package com.kshana.app.data

import android.content.Context
import com.kshana.app.engine.Instants
import org.json.JSONArray
import org.json.JSONObject
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.RemedyRecord

/**
 * One remedy the person holds: the rule it came from, when it was taken up, the exact
 * instants the engine planned for it (each window's start, in epoch millis) and the
 * instants the person logged "did it". [plannedInstants] come from the solved windows
 * and are exact; [doneInstants] come from the device clock at the tap, which [clockSource]
 * states so a reader never mistakes one for the other.
 */
data class Commitment(
    val ruleId: String,
    val title: String,
    val startedAtMillis: Long,
    val cadence: String,
    val plannedInstants: List<Long>,
    val doneInstants: List<Long>,
    val clockSource: String = CommitmentStore.DEVICE_CLOCK,
)

/**
 * The parihara ledger's counts: kept over planned for the last 7 and 40 days, and the
 * planned instants that closed without a log, oldest first. No streaks: a lapse is a
 * row to repair in the next window, not a counter that resets.
 */
data class Metrics(
    val kept7: Int,
    val planned7: Int,
    val kept40: Int,
    val planned40: Int,
    val lapses: List<Long>,
)

/**
 * The parihara ledger: one JSON object per line under `commit.<profileName>` in the
 * shared preferences file. A planned instant is "kept" when a logged instant falls
 * between an hour before it and the earlier of the next planned instant and a day
 * later; it is a "lapse" once that span has closed with nothing logged; until then it
 * is open and counts for nothing. A log with no planned window near it still counts as
 * kept and planned (the act was done; it was not punctual). Everything stays on the phone.
 */
object CommitmentStore {
    /** What [Commitment.doneInstants] are read from. */
    const val DEVICE_CLOCK = "device clock, System.currentTimeMillis() at the tap"

    private const val PREFIX = "commit."
    private const val HOUR_MS = 3_600_000L
    private const val DAY_MS = 86_400_000L
    private const val WEEK_MS = 7L * DAY_MS
    private const val VOW_MS = 40L * DAY_MS

    private fun key(profileName: String): String = "$PREFIX$profileName"

    /** Epoch millis of an exact instant, for alarms and the ledger. */
    fun millisOf(e: ExactInstant): Long = Instants.toLocal(e).toInstant().toEpochMilli()

    private fun toJson(c: Commitment): String = JSONObject()
        .put("ruleId", c.ruleId)
        .put("title", c.title)
        .put("startedAt", c.startedAtMillis)
        .put("cadence", c.cadence)
        .put("planned", JSONArray().also { a -> c.plannedInstants.forEach { a.put(it) } })
        .put("done", JSONArray().also { a -> c.doneInstants.forEach { a.put(it) } })
        .put("clock", c.clockSource)
        .toString()

    private fun longs(a: JSONArray?): List<Long> {
        if (a == null) return emptyList()
        val out = ArrayList<Long>(a.length())
        for (i in 0 until a.length()) out += a.optLong(i)
        return out
    }

    private fun fromJson(line: String): Commitment? = runCatching {
        val o = JSONObject(line)
        Commitment(
            ruleId = o.getString("ruleId"),
            title = o.optString("title", ""),
            startedAtMillis = o.optLong("startedAt", 0L),
            cadence = o.optString("cadence", ""),
            plannedInstants = longs(o.optJSONArray("planned")).sorted(),
            doneInstants = longs(o.optJSONArray("done")).sorted(),
            clockSource = o.optString("clock", DEVICE_CLOCK),
        )
    }.getOrNull()

    /** Every commitment for the profile, oldest first; unreadable lines are dropped. */
    fun commitments(context: Context, profileName: String): List<Commitment> {
        val raw = ProfileStore.prefs(context).getString(key(profileName), null) ?: return emptyList()
        return raw.split('\n').filter { it.isNotBlank() }.mapNotNull(::fromJson).sortedBy { it.startedAtMillis }
    }

    /** Replaces the whole list. */
    fun save(context: Context, profileName: String, list: List<Commitment>) {
        val lines = list.sortedBy { it.startedAtMillis }.joinToString("\n") { toJson(it) }
        ProfileStore.prefs(context).edit().putString(key(profileName), lines).apply()
    }

    /**
     * Holds [remedy]: a new commitment planned from its windows, or, when the rule is
     * already held, the same commitment with any new windows merged in. Returns the list.
     */
    fun commit(context: Context, profileName: String, remedy: RemedyRecord, nowMillis: Long = System.currentTimeMillis()): List<Commitment> {
        val current = commitments(context, profileName)
        val starts = remedy.windows.map { millisOf(it.start) }
        val existing = current.firstOrNull { it.ruleId == remedy.ruleId }
        val next = if (existing == null) {
            current + Commitment(remedy.ruleId, remedy.title, nowMillis, remedy.cadence, starts.distinct().sorted(), emptyList())
        } else {
            current.map { c ->
                if (c.ruleId != remedy.ruleId) c
                else c.copy(plannedInstants = (c.plannedInstants + starts).distinct().sorted())
            }
        }
        save(context, profileName, next)
        return next
    }

    /**
     * Merges the engine's latest windows into every held remedy among [remedies], so the
     * plan keeps growing as the solved windows roll forward. Nothing is held anew.
     */
    fun refreshPlans(context: Context, profileName: String, remedies: List<RemedyRecord>): List<Commitment> {
        val current = commitments(context, profileName)
        if (current.isEmpty()) return current
        var changed = false
        val next = current.map { c ->
            val r = remedies.firstOrNull { it.ruleId == c.ruleId } ?: return@map c
            val merged = (c.plannedInstants + r.windows.map { millisOf(it.start) }).distinct().sorted()
            if (merged.size == c.plannedInstants.size) c else {
                changed = true
                c.copy(plannedInstants = merged)
            }
        }
        if (changed) save(context, profileName, next)
        return next
    }

    /** Logs "did it" for [ruleId] at [atMillis], read from the device clock. Returns the list. */
    fun markDone(context: Context, profileName: String, ruleId: String, atMillis: Long = System.currentTimeMillis()): List<Commitment> {
        val next = commitments(context, profileName).map { c ->
            if (c.ruleId != ruleId) c else c.copy(doneInstants = (c.doneInstants + atMillis).sorted(), clockSource = DEVICE_CLOCK)
        }
        save(context, profileName, next)
        return next
    }

    /** Lets [ruleId] go; its rows leave the ledger. Returns the list. */
    fun release(context: Context, profileName: String, ruleId: String): List<Commitment> {
        val next = commitments(context, profileName).filter { it.ruleId != ruleId }
        save(context, profileName, next)
        return next
    }

    /** The end of the span a planned instant can be kept in: the next planned instant, or a day on. */
    private fun closeOf(planned: List<Long>, i: Int): Long {
        val p = planned[i]
        val next = planned.getOrNull(i + 1) ?: Long.MAX_VALUE
        return minOf(next, p + DAY_MS)
    }

    /** Kept over planned for the last 7 and 40 days up to [nowMillis], and the closed lapses within 40 days. */
    fun metrics(commitments: List<Commitment>, nowMillis: Long): Metrics {
        var kept7 = 0
        var planned7 = 0
        var kept40 = 0
        var planned40 = 0
        val lapses = ArrayList<Long>()
        val weekFrom = nowMillis - WEEK_MS
        val vowFrom = nowMillis - VOW_MS
        for (c in commitments) {
            val planned = c.plannedInstants.sorted()
            val matched = HashSet<Long>()
            for (i in planned.indices) {
                val p = planned[i]
                if (p > nowMillis) continue
                val close = closeOf(planned, i)
                val hit = c.doneInstants.firstOrNull { it >= p - HOUR_MS && it < close }
                val kept = hit != null
                if (hit != null) matched += hit
                val closed = nowMillis >= close
                if (!kept && !closed) continue
                if (p >= vowFrom) {
                    planned40++
                    if (kept) kept40++ else lapses += p
                }
                if (p >= weekFrom) {
                    planned7++
                    if (kept) kept7++
                }
            }
            for (d in c.doneInstants) {
                if (d in matched || d > nowMillis) continue
                if (d >= vowFrom) {
                    planned40++
                    kept40++
                }
                if (d >= weekFrom) {
                    planned7++
                    kept7++
                }
            }
        }
        lapses.sort()
        return Metrics(kept7, planned7, kept40, planned40, lapses)
    }

    /** The soonest planned instant after [nowMillis] across every commitment, if any. */
    fun nextPlanned(commitments: List<Commitment>, nowMillis: Long): Long? =
        commitments.asSequence().flatMap { it.plannedInstants.asSequence() }.filter { it > nowMillis }.minOrNull()
}
