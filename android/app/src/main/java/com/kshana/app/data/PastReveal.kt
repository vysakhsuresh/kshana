package com.kshana.app.data

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import uniffi.kshana_ffi.ExactInstant

/**
 * The person's answers about their past, and the predictions sealed for them. Both
 * live in the profile store, so the backup carries them.
 *
 * Answers sharpen the birth time: a Yes and a No count alike, because a wrong guess
 * still helps, because it narrows down the birth time.
 */
object PastReveal {
    private const val K_ANSWERS = "reveal.answers"
    private const val K_SEALS = "seal.list"

    /** One stretch shown to the person, and what they said ("yes", "close", "no"; "" = not sure). */
    data class Item(
        val fromSec: Long,
        val fromNs: Int,
        val toSec: Long,
        val toNs: Int,
        val life: String,
        val good: Boolean,
        val statement: String,
        val verdict: String,
    ) {
        fun from() = ExactInstant(fromSec, fromNs.toUInt())
        fun to() = ExactInstant(toSec, toNs.toUInt())
    }

    fun answers(context: Context): List<Item> = runCatching {
        val a = JSONArray(ProfileStore.prefs(context).getString(K_ANSWERS, "[]"))
        (0 until a.length()).map { i ->
            val o = a.getJSONObject(i)
            Item(o.getLong("fs"), o.optInt("fn"), o.getLong("ts"), o.optInt("tn"), o.getString("life"), o.getBoolean("good"), o.optString("text"), o.optString("v"))
        }
    }.getOrDefault(emptyList())

    fun saveAnswers(context: Context, items: List<Item>) {
        val a = JSONArray()
        for (x in items) {
            a.put(JSONObject().put("fs", x.fromSec).put("fn", x.fromNs).put("ts", x.toSec).put("tn", x.toNs).put("life", x.life).put("good", x.good).put("text", x.statement).put("v", x.verdict))
        }
        ProfileStore.prefs(context).edit().putString(K_ANSWERS, a.toString()).apply()
    }

    /** Forget the answers (after the birth time itself was changed by hand). */
    fun clear(context: Context) = ProfileStore.prefs(context).edit().remove(K_ANSWERS).remove(K_FACTS).apply()

    private const val K_FACTS = "reveal.facts"

    /** The note written to the journal when a stretch is confirmed here; such entries are already answers. */
    const val CONFIRMED_NOTE = "Confirmed when Kshana read my past"

    /**
     * One fact read from the chart ("Your chart says you are the first-born… Is that
     * right?") and what the person said: "yes", "no", "unsure", or "" not yet answered.
     * [key] is the engine's claim key, passed back so every candidate birth time can be
     * judged against the answer.
     */
    data class Fact(
        val key: String,
        val topic: String,
        val statement: String,
        val sensitive: Boolean,
        val verdict: String,
    )

    fun facts(context: Context): List<Fact> = runCatching {
        val a = JSONArray(ProfileStore.prefs(context).getString(K_FACTS, "[]"))
        (0 until a.length()).map { i ->
            val o = a.getJSONObject(i)
            Fact(o.getString("k"), o.optString("topic"), o.optString("text"), o.optBoolean("s"), o.optString("v"))
        }
    }.getOrDefault(emptyList())

    fun saveFacts(context: Context, facts: List<Fact>) {
        val a = JSONArray()
        for (f in facts) a.put(JSONObject().put("k", f.key).put("topic", f.topic).put("text", f.statement).put("s", f.sensitive).put("v", f.verdict))
        ProfileStore.prefs(context).edit().putString(K_FACTS, a.toString()).apply()
    }

    /** The answered facts as the engine takes them ("not sure" is left out). */
    fun factAnswers(facts: List<Fact>): List<uniffi.kshana_ffi.FactAnswerInput> =
        facts.filter { it.verdict == "yes" || it.verdict == "no" }.map { uniffi.kshana_ffi.FactAnswerInput(it.key, it.verdict) }

    /**
     * Every dated event the person logged themselves, for the birth time: the journal's
     * life events (except those written here when a stretch was confirmed, which are
     * already answers) and the ledger's events. Each is placed at noon of its day.
     */
    fun logged(context: Context, profile: BirthProfile): List<uniffi.kshana_ffi.LoggedEventInput> {
        val zone = profile.zone()
        val fromJournal = Journal.all(context).filter { it.note != CONFIRMED_NOTE }.map { e ->
            uniffi.kshana_ffi.LoggedEventInput(com.kshana.app.engine.Instants.fromInstant(e.date.atTime(12, 0).atZone(zone).toInstant()), e.life, e.good)
        }
        val fromLedger = LedgerStore.events(context, profile.name).map { e ->
            uniffi.kshana_ffi.LoggedEventInput(e.at, e.area, e.kind !in setOf("Loss", "Crisis", "Ended"))
        }
        return fromJournal + fromLedger
    }

    /**
     * A prediction written down and locked: what it says stays hidden until its window
     * has passed, and its fingerprint (SHA-256 of the words, the window and the time it
     * was sealed) shows nothing was changed after sealing.
     */
    data class Seal(
        val sealedAt: Long,
        val fromSec: Long,
        val toSec: Long,
        val life: String,
        val good: Boolean,
        val statement: String,
        val question: String,
        val fingerprint: String,
        /** "" until answered, then "yes" / "close" / "no". */
        val verdict: String,
        /** When it may be opened: the morning after its window ends, in phone time. */
        val opensMs: Long,
    ) {
        /** Opens the morning after the window ends. */
        fun opensAt(): Long = opensMs
        fun isOpen(now: Long = System.currentTimeMillis()) = now >= opensAt()
        fun shortCode() = fingerprint.take(8).uppercase().chunked(4).joinToString("-")
    }

    fun seals(context: Context): List<Seal> = runCatching {
        val a = JSONArray(ProfileStore.prefs(context).getString(K_SEALS, "[]"))
        (0 until a.length()).map { i ->
            val o = a.getJSONObject(i)
            Seal(o.getLong("at"), o.getLong("fs"), o.getLong("ts"), o.getString("life"), o.getBoolean("good"), o.getString("text"), o.getString("q"), o.getString("fp"), o.optString("v"), o.getLong("om"))
        }
    }.getOrDefault(emptyList())

    private fun saveSeals(context: Context, seals: List<Seal>) {
        val a = JSONArray()
        for (s in seals) {
            a.put(JSONObject().put("at", s.sealedAt).put("fs", s.fromSec).put("ts", s.toSec).put("life", s.life).put("good", s.good).put("text", s.statement).put("q", s.question).put("fp", s.fingerprint).put("v", s.verdict).put("om", s.opensMs))
        }
        ProfileStore.prefs(context).edit().putString(K_SEALS, a.toString()).apply()
    }

    /** The seal still waiting to open or to be answered, if any. */
    fun pending(context: Context): Seal? = seals(context).firstOrNull { it.verdict.isEmpty() }

    fun seal(context: Context, from: ExactInstant, to: ExactInstant, life: String, good: Boolean, statement: String, question: String): Seal {
        val at = System.currentTimeMillis()
        val canon = "kshana-seal-v1|$at|${from.taiSeconds}|${to.taiSeconds}|$life|$good|$statement"
        val fp = java.security.MessageDigest.getInstance("SHA-256").digest(canon.toByteArray()).joinToString("") { "%02x".format(it) }
        val opens = com.kshana.app.engine.Instants.toLocal(to).toLocalDate().plusDays(1).atTime(6, 0).atZone(java.time.ZoneId.systemDefault()).toInstant().toEpochMilli()
        val s = Seal(at, from.taiSeconds, to.taiSeconds, life, good, statement, question, fp, "", opens)
        saveSeals(context, seals(context) + s)
        return s
    }

    /** The person says whether the sealed stretch happened; it also joins the answers about the past. */
    fun resolve(context: Context, seal: Seal, verdict: String) {
        saveSeals(context, seals(context).map { if (it.fingerprint == seal.fingerprint) it.copy(verdict = verdict) else it })
        saveAnswers(context, answers(context) + Item(seal.fromSec, 0, seal.toSec, 0, seal.life, seal.good, seal.statement, verdict))
    }

    /** Right, close and asked, over the answered past stretches and sealed predictions. */
    fun score(items: List<Item>): Triple<Int, Int, Int> {
        val answered = items.filter { it.verdict.isNotEmpty() }
        return Triple(answered.count { it.verdict == "yes" }, answered.count { it.verdict == "close" }, answered.size)
    }
}
