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

    fun start(context: Context, title: String, how: String, forWhat: String, days: Int, weekday: Int) {
        val list = all(context).filterNot { it.title == title && !it.ended() }
        save(context, list + Plan(System.currentTimeMillis(), title, how, forWhat, days, LocalDate.now(), weekday, emptySet()))
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
