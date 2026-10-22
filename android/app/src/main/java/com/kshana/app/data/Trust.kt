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
import uniffi.kshana_ffi.FamilyMemberInput
import uniffi.kshana_ffi.PersonalWeightsInput
import uniffi.kshana_ffi.SureWindowRecord
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
            var n = sealFeed(context, profile, firm, room)
            // Every other firm dated feed is sealed here too, in the data layer, so it
            // counts whether or not its page was ever opened: the windows the ways of
            // reading agree on, and the family's agreed windows about the person.
            n += runCatching { sealSureWindows(context, profile, sureWindows(context, profile, settings)) }.getOrDefault(0)
            n += runCatching { sealFeed(context, profile, familyCandidates(context, profile, settings), MAX_FAMILY) }.getOrDefault(0)
            ClaimStore.markTopUp(context, profile.name, nowMs)
            n
        }.getOrDefault(0)
    }

    /** Most agreed windows sealed in one top-up, and most family windows. */
    private const val MAX_SURE = 4
    private const val MAX_FAMILY = 2

    /** The agreed windows of the next six months, read with the person's own earned weights. */
    private suspend fun sureWindows(context: Context, profile: BirthProfile, settings: EngineSettings): List<SureWindowRecord> {
        val engine = EngineProvider.get(context)
        val place = ProfileStore.currentLocation(context)
        return withContext(Dispatchers.Default) {
            engine.sureWindowsTrusted(
                profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa,
                place.latitude, place.longitude, PlaceClock.offsetMinutes(place), Instants.now(), 6u, emptyList(), 30u, Lang.code,
                Integrity.systemVotes(context, profile),
            )
        }
    }

    /**
     * Seal the strong and likely windows of `windows` (from `sureWindows` or
     * `sureWindowsTrusted`) that are not sealed yet, remembering which ways of reading
     * agreed on each so each can earn its own record. Returns how many were sealed.
     */
    fun sealSureWindows(context: Context, profile: BirthProfile, windows: List<SureWindowRecord>, max: Int = MAX_SURE): Int {
        val off = OpenTest.silenced(context, profile).map { it.area }.toSet()
        val firm = windows.filter { (it.tier == "strong" || it.tier == "likely") && it.area !in off }.sortedBy { it.start.taiSeconds }
        val candidates = firm.map { w ->
            ClaimInputRecord(
                source = "consensus", area = w.area, kind = w.kind, good = !w.isHard,
                windowStart = w.start, windowEnd = w.end, probability = if (w.tier == "strong") 0.8 else 0.65,
                statement = w.sealText.ifBlank { w.headline }, question = "",
            )
        }
        return sealFeed(context, profile, candidates, max, firm.map { it.systemKeys })
    }

    /**
     * The family's agreed windows about the person themself (not about the relatives),
     * the firm ones only: three or more charts agreeing, or a clear reading at the peak.
     */
    private suspend fun familyCandidates(context: Context, profile: BirthProfile, settings: EngineSettings): List<ClaimInputRecord> {
        val relations = runCatching { JSONObject(ProfileStore.prefs(context).getString("partners.relations.json", "{}") ?: "{}") }.getOrNull() ?: return emptyList()
        val people = (ProfileStore.partners(context) + FamilyStore.family(context))
            .filter { it.name != profile.name && relations.has(it.name) }
            .distinctBy { it.name }
        if (people.isEmpty()) return emptyList()
        val engine = EngineProvider.get(context)
        val place = ProfileStore.currentLocation(context)
        val male = if (ProfileStore.prefs(context).contains("partners.selfIsGroom")) ProfileStore.selfIsGroom(context) else null
        val r = withContext(Dispatchers.Default) {
            engine.familyProof(
                profile.birthInstant(), profile.latitude, profile.longitude, male,
                people.map { FamilyMemberInput(it.name, relations.optString(it.name, "Other"), it.birthInstant(), it.latitude, it.longitude) },
                place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(), 6u, Lang.code,
            )
        }
        return r.aboutYou
            .filter { w -> w.confirmedBy.size >= 3 || (if (w.isHard) w.strength <= 30.0 else w.strength >= 70.0) }
            .mapNotNull { w ->
                val life = uniffi.kshana_ffi.lifeKeyOf(w.area) ?: return@mapNotNull null
                ClaimInputRecord(
                    source = "family", area = life, kind = w.area, good = !w.isHard,
                    windowStart = w.start, windowEnd = w.end, probability = if (w.confirmedBy.size >= 3) 0.75 else 0.65,
                    statement = w.headline, question = "",
                )
            }
    }

    /**
     * Seal a prashna answer that names a time: a Yes, with its timing windows, as one
     * claim from the first window that opens a day or more ahead to the last. A No, an
     * unclear answer or one with no dated window is not a dated prediction and is not
     * sealed. Returns whether it was sealed.
     */
    fun sealPrashna(context: Context, area: String, rec: uniffi.kshana_ffi.PrashnaRecord): Boolean = runCatching {
        if (rec.verdict != "Yes") return@runCatching false
        val profile = ProfileStore.load(context) ?: return@runCatching false
        val life = uniffi.kshana_ffi.lifeKeyOf(area) ?: return@runCatching false
        val lead = Instants.now().taiSeconds + 24 * 3600
        val ahead = rec.windows.filter { it.start.taiSeconds >= lead }
        val first = ahead.minByOrNull { it.start.taiSeconds } ?: return@runCatching false
        val last = ahead.maxByOrNull { it.end.taiSeconds } ?: return@runCatching false
        val name = com.kshana.app.ui.screens.lifeName(life).lowercase()
        val candidate = ClaimInputRecord(
            source = "prashna", area = life, kind = "question", good = true,
            windowStart = first.start, windowEnd = last.end, probability = 0.6,
            statement = "The question you asked about $name: it is likely to come, at the times found for it.", question = "",
        )
        sealFeed(context, profile, listOf(candidate), 1) > 0
    }.getOrDefault(false)

    /**
     * Seal up to `max` of `candidates` (any dated prediction: area, direction, window,
     * probability, words) that are not already sealed, and store them. The same
     * prediction is never sealed twice, and nothing whose window has opened is sealed.
     * `systems`, when given, runs beside `candidates`: the ways of reading that agreed
     * on each, kept beside the claim (never inside its fingerprint).
     */
    fun sealFeed(context: Context, profile: BirthProfile, candidates: List<ClaimInputRecord>, max: Int, systems: List<List<String>> = emptyList()): Int {
        if (max <= 0 || candidates.isEmpty()) return 0
        // The stretch sealed on the past-check page is a sealed prediction too.
        val stretches = PastReveal.seals(context).filter { it.verdict.isEmpty() }
        val fresh = candidates.filter { c ->
            stretches.none { s -> s.life.equals(c.area, true) && s.good == c.good && s.fromSec < c.windowEnd.taiSeconds && c.windowStart.taiSeconds < s.toSec }
        }
        val sealed = sealNewClaims(ClaimStore.lines(context, profile.name), fresh, Instants.now(), max.toUInt(), Lang.code, profile.longitude)
        ClaimStore.addAll(context, profile.name, sealed.map { it.line })
        if (systems.isNotEmpty()) {
            val meta = sealed.mapNotNull { c ->
                val i = candidates.indexOfFirst { it.area == c.area && it.good == c.good && it.windowStart.taiSeconds == c.windowStart.taiSeconds && it.windowEnd.taiSeconds == c.windowEnd.taiSeconds }
                systems.getOrNull(i)?.takeIf { it.isNotEmpty() }?.let { c.key to it }
            }.toMap()
            ClaimMeta.put(context, profile.name, meta)
        }
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

    /**
     * Every answered item, each flagged by the hold-out rule. The headline counts only
     * sealed predictions answered after their dates (`usedForFitting = false`: they were
     * scored blind, and their score is frozen in the stored line; joining the birth-time
     * narrowing afterwards never re-counts them). Answers about the past tuned the birth
     * time and the person's weights before anything scored them, so they are flagged and
     * kept on their own separate line; the quiz is about the past too and the engine keeps
     * it there as well. Logged events (the journal, the ledger) tune the birth time and
     * never enter the score at all.
     */
    fun scoreItems(context: Context, profile: BirthProfile): List<ScoreItemInput> {
        val out = ArrayList<ScoreItemInput>()
        for (c in ClaimStore.entries(context, profile.name)) {
            if (c.outcome.isNotEmpty()) out.add(ScoreItemInput("sealed", c.outcome, 0.5, false))
        }
        val stretches = PastReveal.seals(context).filter { it.verdict.isNotEmpty() }
        for (s in stretches) out.add(ScoreItemInput("sealed", s.verdict, 0.5, false))
        for (a in PastReveal.answers(context)) {
            if (a.verdict !in setOf("yes", "close", "no")) continue
            // A sealed prediction, once answered, also joins the answers: it is counted once, as sealed.
            if (a.sealedId.isNotEmpty()) continue
            if (stretches.any { it.fromSec == a.fromSec && it.toSec == a.toSec && it.life == a.life }) continue
            out.add(ScoreItemInput("past", a.verdict, 0.5, true))
        }
        for (g in QuizScores.all(context, profile.name)) {
            repeat(g.total) { i -> out.add(ScoreItemInput("quiz", if (i < g.correct) "yes" else "no", g.chance, false)) }
        }
        return out
    }
}
