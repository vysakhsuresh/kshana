package com.kshana.app.data

import android.content.Context
import android.content.Intent
import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject
import java.time.LocalDate
import java.time.temporal.ChronoUnit

/** A remedy the person has taken on for a set number of days. */
data class Plan(
    val id: Long,
    val title: String,
    val how: String,
    val forWhat: String,
    val days: Int,
    val start: LocalDate,
    /** ISO day-of-week 1–7 when the remedy is weekly; 0 for every day. */
    val weekday: Int,
    val done: Set<LocalDate>,
) {
    fun dueOn(d: LocalDate): Boolean = !d.isBefore(start) && ChronoUnit.DAYS.between(start, d) < days && (weekday == 0 || d.dayOfWeek.value == weekday)
    fun dueCount(): Int = (0 until days).count { dueOn(start.plusDays(it.toLong())) }
    fun doneCount(): Int = done.count { dueOn(it) }
    fun ended(today: LocalDate = LocalDate.now()) = ChronoUnit.DAYS.between(start, today) >= days
    fun dayNumber(today: LocalDate = LocalDate.now()) = (ChronoUnit.DAYS.between(start, today) + 1).coerceIn(1, days.toLong()).toInt()
}

/**
 * Remedy plans: 21 or 40 days of one practice, with a tick for each day it was done.
 * The sunrise reading names today's practices. Kept on the phone and in the backup.
 */
object RemedyPlans {
    private const val KEY = "plans.json"

    fun all(context: Context): List<Plan> {
        val raw = ProfileStore.prefs(context).getString(KEY, null) ?: return emptyList()
        return runCatching {
            val a = JSONArray(raw)
            (0 until a.length()).map { i ->
                val o = a.getJSONObject(i)
                val d = o.getJSONArray("done")
                Plan(
                    o.getLong("id"), o.getString("title"), o.optString("how"), o.optString("for"), o.getInt("days"),
                    LocalDate.parse(o.getString("start")), o.optInt("weekday", 0),
                    (0 until d.length()).map { LocalDate.parse(d.getString(it)) }.toSet(),
                )
            }
        }.getOrDefault(emptyList())
    }

    private fun save(context: Context, list: List<Plan>) {
        val a = JSONArray()
        for (p in list) a.put(
            JSONObject().put("id", p.id).put("title", p.title).put("how", p.how).put("for", p.forWhat).put("days", p.days)
                .put("start", p.start.toString()).put("weekday", p.weekday).put("done", JSONArray(p.done.map { it.toString() })),
        )
        ProfileStore.prefs(context).edit().putString(KEY, a.toString()).apply()
    }

    fun start(context: Context, title: String, how: String, forWhat: String, days: Int, weekday: Int, from: LocalDate = LocalDate.now()) {
        val list = all(context).filterNot { it.title == title && !it.ended() }
        save(context, list + Plan(System.currentTimeMillis(), title, how, forWhat, days, from, weekday, emptySet()))
    }

    fun toggleToday(context: Context, id: Long) {
        val today = LocalDate.now()
        save(context, all(context).map { if (it.id != id) it else it.copy(done = if (today in it.done) it.done - today else it.done + today) })
    }

    fun stop(context: Context, id: Long) = save(context, all(context).filter { it.id != id })

    fun active(context: Context): List<Plan> = all(context).filter { !it.ended() }

    /** Today's practices, for the sunrise reading. */
    fun todayLine(context: Context): String? {
        val t = LocalDate.now()
        val due = active(context).filter { it.dueOn(t) && t !in it.done }
        return if (due.isEmpty()) null else "Today: " + due.joinToString("; ") { "${it.title} (day ${it.dayNumber()} of ${it.days})" }
    }

    // Giving for real: a UPI ID the person chose, or a well-known food charity.
    private const val K_UPI = "giving.upi"
    private const val K_UPI_NAME = "giving.upi.name"
    fun upi(context: Context): String = ProfileStore.prefs(context).getString(K_UPI, "") ?: ""
    fun upiName(context: Context): String = ProfileStore.prefs(context).getString(K_UPI_NAME, "") ?: ""
    fun setUpi(context: Context, id: String, name: String) =
        ProfileStore.prefs(context).edit().putString(K_UPI, id.trim()).putString(K_UPI_NAME, name.trim()).apply()

    /** Open the person's UPI app to pay their chosen cause, or a food charity's page when none is set. */
    fun giveIntent(context: Context, note: String): Intent {
        val id = upi(context)
        return if (id.contains("@")) {
            val u = Uri.Builder().scheme("upi").authority("pay")
                .appendQueryParameter("pa", id)
                .appendQueryParameter("pn", upiName(context).ifBlank { "Charity" })
                .appendQueryParameter("tn", note.take(50))
                .appendQueryParameter("cu", "INR").build()
            Intent.createChooser(Intent(Intent.ACTION_VIEW, u), "Give with")
        } else {
            Intent(Intent.ACTION_VIEW, Uri.parse("https://www.akshayapatra.org/"))
        }
    }
}

/**
 * A remedy the person took up at its planned hour, and, once its stretch or its days are
 * over, how they said things went. [answer] is one of [FollowThrough.EASED],
 * [FollowThrough.SAME], [FollowThrough.HARDER], [FollowThrough.SKIPPED], or null while
 * the question is still to come.
 */
data class TakenRemedy(
    val remedyId: String,
    val title: String,
    val startMillis: Long,
    val untilMillis: Long,
    /** When to ask "did things ease?". */
    val checkInMillis: Long,
    val stretchEndMillis: Long?,
    val forStretch: String,
    val keepWords: String,
    val takenAtMillis: Long,
    val answer: String? = null,
    val answeredAtMillis: Long? = null,
)

/** The honest record: how many remedies were kept and what the person saw while keeping them. */
data class FollowThroughRecord(val kept: Int, val eased: Int, val same: Int, val harder: Int, val skipped: Int)

/**
 * Remedies with follow-through, per profile: what was taken up and when, and the person's
 * own word afterwards on whether things eased. The app never claims a remedy caused
 * anything; the record says only what the person saw while keeping it. On the phone and
 * in the backup, under `followthrough.<profileName>`.
 */
object FollowThrough {
    const val EASED = "eased"
    const val SAME = "same"
    const val HARDER = "harder"
    const val SKIPPED = "skipped"

    private const val PREFIX = "followthrough."
    private const val DAY_MS = 86_400_000L

    private fun key(profileName: String) = "$PREFIX$profileName"

    fun all(context: Context, profileName: String): List<TakenRemedy> {
        val raw = ProfileStore.prefs(context).getString(key(profileName), null) ?: return emptyList()
        return runCatching {
            val a = JSONArray(raw)
            (0 until a.length()).map { i ->
                val o = a.getJSONObject(i)
                TakenRemedy(
                    remedyId = o.getString("id"),
                    title = o.optString("title"),
                    startMillis = o.optLong("start"),
                    untilMillis = o.optLong("until"),
                    checkInMillis = o.optLong("checkIn"),
                    stretchEndMillis = if (o.has("stretchEnd")) o.getLong("stretchEnd") else null,
                    forStretch = o.optString("for"),
                    keepWords = o.optString("keep"),
                    takenAtMillis = o.optLong("takenAt"),
                    answer = if (o.has("answer")) o.getString("answer") else null,
                    answeredAtMillis = if (o.has("answeredAt")) o.getLong("answeredAt") else null,
                )
            }
        }.getOrDefault(emptyList())
    }

    private fun save(context: Context, profileName: String, list: List<TakenRemedy>) {
        val a = JSONArray()
        for (t in list) a.put(
            JSONObject().put("id", t.remedyId).put("title", t.title).put("start", t.startMillis).put("until", t.untilMillis)
                .put("checkIn", t.checkInMillis).put("for", t.forStretch).put("keep", t.keepWords).put("takenAt", t.takenAtMillis)
                .also { o -> t.stretchEndMillis?.let { o.put("stretchEnd", it) } }
                .also { o -> t.answer?.let { o.put("answer", it) } }
                .also { o -> t.answeredAtMillis?.let { o.put("answeredAt", it) } },
        )
        ProfileStore.prefs(context).edit().putString(key(profileName), a.toString()).apply()
    }

    /** The open (unanswered) entry for [remedyId], if the person holds it now. */
    fun open(context: Context, profileName: String, remedyId: String): TakenRemedy? =
        all(context, profileName).firstOrNull { it.remedyId == remedyId && it.answer == null }

    /**
     * Takes up a scheduled remedy: records it for the check-in, adds it to the daily
     * practices (so the sunrise reading names it on its days) and sets a gentle reminder
     * for the check-in. Taking the same remedy again replaces the open entry.
     */
    fun take(context: Context, profileName: String, s: uniffi.kshana_ffi.RemedyScheduleRecord, now: Long = System.currentTimeMillis()) {
        val list = all(context, profileName).filterNot { it.remedyId == s.remedyId && it.answer == null }
        val t = TakenRemedy(
            remedyId = s.remedyId, title = s.title, startMillis = s.startMs, untilMillis = s.untilMs, checkInMillis = s.checkInMs,
            stretchEndMillis = s.stretchEndMs, forStretch = s.forStretchWords, keepWords = s.keepWords, takenAtMillis = now,
        )
        save(context, profileName, list + t)
        val every = s.everyDays.toInt()
        if (every == 0 || every == 1 || every == 7) {
            val from = java.time.Instant.ofEpochMilli(s.startMs).atZone(java.time.ZoneId.systemDefault()).toLocalDate()
            RemedyPlans.start(context, s.title, s.keepWords, s.forStretchWords, s.days.toInt().coerceAtLeast(1), if (every == 7) s.weekdayIso.toInt() else 0, from)
        }
        com.kshana.app.notify.Reminders.schedule(
            context, com.kshana.app.notify.Reminders.stableId("checkin:$profileName:${s.remedyId}"), s.checkInMs,
            "Did things ease?", "You kept “${s.title}”. Open Kshana to say how it went.",
        )
    }

    /** Lets an open remedy go without an answer (the person stopped it before the end). */
    fun drop(context: Context, profileName: String, remedyId: String) {
        val list = all(context, profileName)
        list.firstOrNull { it.remedyId == remedyId && it.answer == null }?.let { t ->
            RemedyPlans.active(context).filter { it.title == t.title }.forEach { RemedyPlans.stop(context, it.id) }
        }
        save(context, profileName, list.filterNot { it.remedyId == remedyId && it.answer == null })
        com.kshana.app.notify.Reminders.cancel(context, com.kshana.app.notify.Reminders.stableId("checkin:$profileName:$remedyId"))
    }

    /** The person's word on an open remedy. */
    fun answer(context: Context, profileName: String, remedyId: String, answer: String, now: Long = System.currentTimeMillis()) {
        save(context, profileName, all(context, profileName).map {
            if (it.remedyId == remedyId && it.answer == null) it.copy(answer = answer, answeredAtMillis = now) else it
        })
    }

    /** "Ask me later": the question comes back in three days. */
    fun later(context: Context, profileName: String, remedyId: String, now: Long = System.currentTimeMillis()) {
        save(context, profileName, all(context, profileName).map {
            if (it.remedyId == remedyId && it.answer == null) it.copy(checkInMillis = now + 3 * DAY_MS) else it
        })
    }

    /** Check-ins due now, oldest first. */
    fun due(context: Context, profileName: String, now: Long = System.currentTimeMillis()): List<TakenRemedy> =
        all(context, profileName).filter { it.answer == null && now >= it.checkInMillis }.sortedBy { it.checkInMillis }

    fun record(list: List<TakenRemedy>): FollowThroughRecord {
        val answered = list.filter { it.answer != null }
        return FollowThroughRecord(
            kept = answered.count { it.answer != SKIPPED },
            eased = answered.count { it.answer == EASED },
            same = answered.count { it.answer == SAME },
            harder = answered.count { it.answer == HARDER },
            skipped = answered.count { it.answer == SKIPPED },
        )
    }

    /** "You kept 3 remedies; 2 eased, 1 the same." Null before the first answer. */
    fun recordWords(r: FollowThroughRecord): String? {
        if (r.kept == 0 && r.skipped == 0) return null
        if (r.kept == 0) return "You have not kept one through yet. That is fine; the next one can start small."
        val parts = buildList {
            if (r.eased > 0) add("${r.eased} eased")
            if (r.same > 0) add("${r.same} the same")
            if (r.harder > 0) add("${r.harder} harder")
        }
        val n = if (r.kept == 1) "1 remedy" else "${r.kept} remedies"
        return "You kept $n; ${parts.joinToString(", ")}."
    }
}
