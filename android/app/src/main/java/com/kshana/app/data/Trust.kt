package com.kshana.app.data

import android.content.Context
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import uniffi.kshana_ffi.ClaimInputRecord
import uniffi.kshana_ffi.PersonalWeightsInput
import uniffi.kshana_ffi.ScoreItemInput
import uniffi.kshana_ffi.TechniqueLearnRecord
import uniffi.kshana_ffi.TechniqueWeightInput
import uniffi.kshana_ffi.sealNewClaims

/**
 * The person's learned weights (which way of reading fits their past), kept per profile
 * so the forecast can lean on them: the `answers` and per-technique standing of the last
 * `learnTechniques` result. The engine decides whether they are enough to move anything.
 */
object PersonalWeights {
    private const val PREFIX = "weights."

    fun save(context: Context, profileName: String, r: TechniqueLearnRecord) {
        val a = JSONArray()
        for (t in r.techniques) a.put(JSONObject().put("k", t.key).put("w", t.weight).put("s", t.skill).put("g", t.logGain))
        ProfileStore.prefs(context).edit().putString(PREFIX + profileName, JSONObject().put("n", r.answers.toLong()).put("t", a).toString()).apply()
    }

    /** The stored weights; no answers (the usual forecast) when none are stored. */
    fun input(context: Context, profileName: String): PersonalWeightsInput = runCatching {
        val o = JSONObject(ProfileStore.prefs(context).getString(PREFIX + profileName, null) ?: return@runCatching null)
        val a = o.getJSONArray("t")
        PersonalWeightsInput(
            o.getLong("n").toUInt(),
            (0 until a.length()).map { i -> a.getJSONObject(i).let { TechniqueWeightInput(it.getString("k"), it.getDouble("w"), it.getDouble("s"), it.getDouble("g")) } },
        )
    }.getOrNull() ?: PersonalWeightsInput(0u, emptyList())

    fun clear(context: Context, profileName: String) = ProfileStore.prefs(context).edit().remove(PREFIX + profileName).apply()
}

/** The which-window quiz grades, kept as counts only (never the answers), one per quiz set. */
object QuizScores {
    private const val PREFIX = "quiz.grades."

    data class Grade(val set: String, val correct: Int, val total: Int, val chance: Double)

    fun all(context: Context, profileName: String): List<Grade> = runCatching {
        val a = JSONArray(ProfileStore.prefs(context).getString(PREFIX + profileName, "[]"))
        (0 until a.length()).map { i -> a.getJSONObject(i).let { Grade(it.getString("id"), it.getInt("c"), it.getInt("n"), it.getDouble("p")) } }
    }.getOrDefault(emptyList())

    /** Records the grade of one quiz set; grading the same set again replaces it. */
    fun record(context: Context, profileName: String, set: String, correct: Int, total: Int, chance: Double) {
        if (total <= 0) return
        val list = all(context, profileName).filter { it.set != set } + Grade(set, correct, total, chance)
        val a = JSONArray()
        for (g in list.takeLast(100)) a.put(JSONObject().put("id", g.set).put("c", g.correct).put("n", g.total).put("p", g.chance))
        ProfileStore.prefs(context).edit().putString(PREFIX + profileName, a.toString()).apply()
    }
}

/**
 * The trust loop: firm predictions sealed before their windows open, answered once they
 * close, and one plain score over everything answered.
 */
object Trust {
    /** Sealed predictions kept waiting at any time. */
    const val KEEP_WAITING = 3
    private const val WEEK_MS = 7L * 24 * 3600 * 1000
    private val lock = Mutex()

    /** Whether a top-up is due: a week since the last, or fewer than [KEEP_WAITING] still sealed. */
    fun topUpDue(context: Context, profile: BirthProfile, nowMs: Long = System.currentTimeMillis()): Boolean {
        val waiting = ClaimStore.waiting(context, profile.name, Instants.now().taiSeconds).size
        return waiting < KEEP_WAITING || nowMs - ClaimStore.lastTopUp(context, profile.name) >= WEEK_MS
    }

    /**
     * Seal the forecast's firm windows that are not sealed yet, so a few always wait:
     * up to [KEEP_WAITING] minus those waiting, and at least one new one a week. Any
     * other dated feed can be sealed the same way with [sealFeed]. Returns how many were
     * sealed. Never throws.
     */
    suspend fun topUp(context: Context, profile: BirthProfile, settings: EngineSettings = ProfileStore.settings(context), force: Boolean = false): Int = lock.withLock {
        runCatching {
            val nowMs = System.currentTimeMillis()
            if (!force && !topUpDue(context, profile, nowMs)) return@runCatching 0
            val engine = EngineProvider.get(context)
            val now = Instants.now()
            val firm = withContext(Dispatchers.Default) {
                engine.firmPredictions(
                    profile.birthInstant(), profile.latitude, profile.longitude, profile.latitude, profile.longitude,
                    settings.houseSystem, settings.ayanamsa, now, Lang.code, PersonalWeights.input(context, profile.name),
                )
            }
            val waiting = ClaimStore.waiting(context, profile.name, now.taiSeconds).size
            val weekly = nowMs - ClaimStore.lastTopUp(context, profile.name) >= WEEK_MS
            val room = maxOf(KEEP_WAITING - waiting, if (weekly) 1 else 0)
            val n = sealFeed(context, profile, firm, room)
            ClaimStore.markTopUp(context, profile.name, nowMs)
            n
        }.getOrDefault(0)
    }

    /**
     * Seal up to `max` of `candidates` (any dated prediction: area, direction, window,
     * probability, words) that are not already sealed, and store them. The same
     * prediction is never sealed twice, and nothing whose window has opened is sealed.
     */
    fun sealFeed(context: Context, profile: BirthProfile, candidates: List<ClaimInputRecord>, max: Int): Int {
        if (max <= 0 || candidates.isEmpty()) return 0
        // The stretch sealed on the past-check page is a sealed prediction too.
        val stretches = PastReveal.seals(context).filter { it.verdict.isEmpty() }
        val fresh = candidates.filter { c ->
            stretches.none { s -> s.life.equals(c.area, true) && s.good == c.good && s.fromSec < c.windowEnd.taiSeconds && c.windowStart.taiSeconds < s.toSec }
        }
        val sealed = sealNewClaims(ClaimStore.lines(context, profile.name), fresh, Instants.now(), max.toUInt(), Lang.code, profile.longitude)
        ClaimStore.addAll(context, profile.name, sealed.map { it.line })
        return sealed.size
    }

    private const val UNSURE = "claims.unsure."

    /** The person was not sure about a closed claim: it is never asked again and stays out of the score. */
    fun markUnsure(context: Context, profile: BirthProfile, id: String) {
        val p = ProfileStore.prefs(context)
        val set = p.getStringSet(UNSURE + profile.name, emptySet()).orEmpty() + id
        p.edit().putStringSet(UNSURE + profile.name, set).apply()
    }

    fun isUnsure(context: Context, profile: BirthProfile, id: String): Boolean =
        id in ProfileStore.prefs(context).getStringSet(UNSURE + profile.name, emptySet()).orEmpty()

    /** Every answered item: sealed predictions (both kinds), the past, the quiz. */
    fun scoreItems(context: Context, profile: BirthProfile): List<ScoreItemInput> {
        val out = ArrayList<ScoreItemInput>()
        for (c in ClaimStore.entries(context, profile.name)) {
            if (c.outcome.isNotEmpty()) out.add(ScoreItemInput("sealed", c.outcome, 0.5))
        }
        val stretches = PastReveal.seals(context).filter { it.verdict.isNotEmpty() }
        for (s in stretches) out.add(ScoreItemInput("sealed", s.verdict, 0.5))
        for (a in PastReveal.answers(context)) {
            if (a.verdict !in setOf("yes", "close", "no")) continue
            // A sealed stretch, once answered, also joins the answers: count it once, as sealed.
            if (stretches.any { it.fromSec == a.fromSec && it.toSec == a.toSec && it.life == a.life }) continue
            out.add(ScoreItemInput("past", a.verdict, 0.5))
        }
        for (g in QuizScores.all(context, profile.name)) {
            repeat(g.total) { i -> out.add(ScoreItemInput("quiz", if (i < g.correct) "yes" else "no", g.chance)) }
        }
        return out
    }
}
