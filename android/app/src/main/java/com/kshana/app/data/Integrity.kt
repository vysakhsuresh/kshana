package com.kshana.app.data

import android.content.Context
import com.kshana.app.engine.Instants
import org.json.JSONArray
import org.json.JSONObject
import uniffi.kshana_ffi.BlindMatchRecord
import uniffi.kshana_ffi.BlindResolveRecord
import uniffi.kshana_ffi.ClaimRecord
import uniffi.kshana_ffi.ConfidenceRecord
import uniffi.kshana_ffi.DaySignalInput
import uniffi.kshana_ffi.RecordCardRecord
import uniffi.kshana_ffi.RemedyCompareRecord
import uniffi.kshana_ffi.SystemRateRecord
import uniffi.kshana_ffi.SystemVoteInput
import java.time.LocalDate

/**
 * The sealed predictions whose answer has joined the birth-time narrowing. A sealed
 * prediction is scored the moment it is answered (its answer is fixed in its stored
 * line); only then may it help tune the birth time. This keeps the record of that use,
 * so it is plain which answers were used after scoring and that none is used twice.
 * Kept in the profile store, so the backup carries it.
 */
object RefineUse {
    private const val KEY = "refine.used"

    fun all(context: Context): Set<String> = ProfileStore.prefs(context).getStringSet(KEY, emptySet()).orEmpty()

    fun record(context: Context, id: String) {
        if (id.isBlank()) return
        val p = ProfileStore.prefs(context)
        p.edit().putStringSet(KEY, all(context) + id).apply()
    }
}

/**
 * What a sealed claim was sealed from, kept beside the claim and never inside its
 * fingerprint: the ways of reading that agreed on it (by key). Lets each way of reading
 * earn its own record from the person's resolved sealed predictions. Under
 * `claims.meta.<profileName>`, a JSON object from claim key to a list of keys.
 */
object ClaimMeta {
    private const val PREFIX = "claims.meta."

    private fun read(context: Context, profileName: String): JSONObject =
        runCatching { JSONObject(ProfileStore.prefs(context).getString(PREFIX + profileName, "{}") ?: "{}") }.getOrDefault(JSONObject())

    fun systems(context: Context, profileName: String, claimKey: String): List<String> = runCatching {
        val a = read(context, profileName).optJSONArray(claimKey) ?: return@runCatching emptyList()
        (0 until a.length()).map { a.getString(it) }
    }.getOrDefault(emptyList())

    fun put(context: Context, profileName: String, entries: Map<String, List<String>>) {
        if (entries.isEmpty()) return
        val o = read(context, profileName)
        for ((k, v) in entries) if (!o.has(k)) o.put(k, JSONArray(v))
        ProfileStore.prefs(context).edit().putString(PREFIX + profileName, o.toString()).apply()
    }
}

/**
 * The record kept honest, on the phone: blind answers to sealed predictions, blind
 * logging with the match shown only after saving, each way of reading's earned record,
 * confidence numbers only when earned, kept weeks beside lapsed weeks for a remedy, and
 * the short record for the About page. The rules themselves live in the engine.
 */
object Integrity {
    /** A sealed claim has just been answered (and so scored): it may now help tune the birth time, once. */
    fun afterScored(context: Context, c: ClaimRecord) {
        if (c.outcome !in setOf("yes", "close", "no")) return
        val life = uniffi.kshana_ffi.lifeKeyOf(c.area) ?: return
        val answers = PastReveal.answers(context)
        if (answers.any { it.sealedId == c.id }) return
        PastReveal.saveAnswers(
            context,
            answers + PastReveal.Item(c.windowStart.taiSeconds, c.windowStart.nanos.toInt(), c.windowEnd.taiSeconds, c.windowEnd.nanos.toInt(), life, c.good, c.statement, c.outcome, c.id),
        )
        RefineUse.record(context, c.id)
    }

    /**
     * Answer a closed claim blind: the person said what happened (`answer`: "good",
     * "hard", "mixed", "nothing", "unsure") before seeing the sealed words. Stores the
     * answered line, then lets the scored answer help tune the birth time. Null on failure.
     */
    fun answerBlind(context: Context, profile: BirthProfile, c: ClaimRecord, answer: String): BlindResolveRecord? = runCatching {
        val r = uniffi.kshana_ffi.blindResolve(c.line, answer, Instants.now(), Lang.code, profile.longitude)
        val answered = r.claim
        if (answered == null) {
            Trust.markUnsure(context, profile, c.id)
        } else {
            ClaimStore.replace(context, profile.name, c.line, answered.line)
            afterScored(context, answered)
        }
        r
    }.getOrNull()

    /**
     * Blind logging: the person writes what happened (part of life, day, good or hard, a
     * line) without seeing any prediction. It is saved to the journal first; only then
     * are the sealed claims it bears on looked up and returned to show ("This was sealed
     * on 21 Oct: …"). A closed, unanswered claim the event agrees with is answered by it
     * (it was written blind); others wait for their own question.
     */
    fun logBlind(context: Context, profile: BirthProfile, date: LocalDate, life: String, good: Boolean, note: String): List<BlindMatchRecord> {
        Journal.add(context, date, life, good, note)
        return runCatching {
            val at = Instants.fromInstant(date.atTime(12, 0).atZone(profile.zone()).toInstant())
            val now = Instants.now()
            val matches = uniffi.kshana_ffi.matchLoggedEvent(ClaimStore.lines(context, profile.name), life, good, at, now, Lang.code, profile.longitude)
            for (m in matches) {
                val c = m.claim
                if (m.agrees && c.outcome.isEmpty() && c.windowEnd.taiSeconds <= now.taiSeconds && !Trust.isUnsure(context, profile, c.id)) {
                    answerBlind(context, profile, c, if (good) "good" else "hard")
                }
            }
            matches
        }.getOrDefault(emptyList())
    }

    /** One vote per way of reading per resolved sealed claim it agreed on. */
    fun systemVotes(context: Context, profile: BirthProfile): List<SystemVoteInput> =
        ClaimStore.entries(context, profile.name).filter { it.outcome.isNotEmpty() }.flatMap { c ->
            ClaimMeta.systems(context, profile.name, c.key).map { SystemVoteInput(it, c.outcome) }
        }

    /** Each way of reading's own record for the person; empty before any vote. */
    fun systemRates(context: Context, profile: BirthProfile): List<SystemRateRecord> =
        runCatching { uniffi.kshana_ffi.systemHitRates(systemVotes(context, profile), Lang.code) }.getOrDefault(emptyList())

    /** Points and count of the person's answered sealed predictions of a tier. */
    fun tierRecord(context: Context, profile: BirthProfile, tier: String): Pair<Double, Int> {
        val mine = ClaimStore.entries(context, profile.name).filter { it.outcome.isNotEmpty() && uniffi.kshana_ffi.tierOfProbability(it.probability) == tier }
        val points = mine.sumOf { when (it.outcome) { "yes" -> 1.0; "close" -> 0.5; else -> 0.0 } }
        return points to mine.size
    }

    /** How sure a firm prediction of `tier` is: a number only when earned. */
    fun confidence(context: Context, profile: BirthProfile, tier: String): ConfidenceRecord? = runCatching {
        val (p, n) = tierRecord(context, profile, tier)
        uniffi.kshana_ffi.confidenceLine(tier, p, n.toUInt(), Lang.code)
    }.getOrNull()

    /** Kshana's record for the person: the headline (sealed predictions only) against chance. */
    fun record(context: Context, profile: BirthProfile): RecordCardRecord? = runCatching {
        val nowTai = Instants.now().taiSeconds
        val claims = ClaimStore.entries(context, profile.name)
        val seals = PastReveal.seals(context)
        val sealed = claims.size + seals.size
        val waiting = claims.count { it.outcome.isEmpty() && it.windowEnd.taiSeconds > nowTai } + seals.count { it.verdict.isEmpty() && !it.isOpen() }
        uniffi.kshana_ffi.recordCard(Trust.scoreItems(context, profile), sealed.toUInt(), waiting.toUInt(), Lang.code)
    }.getOrNull()

    /**
     * The person's own record by day: what they logged (good or hard), and each sealed
     * prediction they answered, on its middle day (a good stretch that came, or a hard
     * one that did not, is good; Close says neither).
     */
    fun daySignals(context: Context, profile: BirthProfile): List<DaySignalInput> {
        val zone = profile.zone()
        val out = ArrayList<DaySignalInput>()
        for (e in Journal.all(context)) if (e.note != PastReveal.CONFIRMED_NOTE) out.add(DaySignalInput(e.date.toEpochDay(), e.good))
        for (c in ClaimStore.entries(context, profile.name)) {
            if (c.outcome != "yes" && c.outcome != "no") continue
            val mid = (c.windowStart.taiSeconds + c.windowEnd.taiSeconds) / 2
            val day = Instants.toLocal(uniffi.kshana_ffi.ExactInstant(mid, 0u), zone).toLocalDate().toEpochDay()
            out.add(DaySignalInput(day, (c.outcome == "yes") == c.good))
        }
        return out
    }

    /** One remedy plan's kept weeks beside its lapsed weeks. */
    fun remedyCompare(context: Context, profile: BirthProfile, plan: Plan): RemedyCompareRecord? = runCatching {
        val due = (0 until plan.days).map { plan.start.plusDays(it.toLong()) }.filter { plan.dueOn(it) }.map { it.toEpochDay() }
        uniffi.kshana_ffi.remedyKeptCompare(due, plan.done.map { it.toEpochDay() }, daySignals(context, profile), LocalDate.now().toEpochDay(), Lang.code)
    }.getOrNull()

    /**
     * Where a remedy stands for the person, by its title: "helps", "no_difference" or
     * "too_early", over every plan of it. For ordering a list of remedies so those that
     * seem to help come first.
     */
    fun remedyStanding(context: Context, profile: BirthProfile, title: String): String {
        val plans = RemedyPlans.all(context).filter { it.title == title }
        val standings = plans.mapNotNull { remedyCompare(context, profile, it)?.standing }
        return when {
            "helps" in standings -> "helps"
            "no_difference" in standings -> "no_difference"
            else -> "too_early"
        }
    }
}
