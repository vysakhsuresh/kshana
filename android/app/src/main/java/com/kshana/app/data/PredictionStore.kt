package com.kshana.app.data

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import uniffi.kshana_ffi.RecordInput

/**
 * The windows the app named, kept so the person can say afterwards whether they came
 * true. One entry per life and window start; at most forty per life. Everything stays
 * on the phone, and the record feeds `calibrate()` in the engine.
 */
data class Prediction(
    val life: String,
    /** Epoch millis of the window's start and end. */
    val start: Long,
    val end: Long,
    /** 0–100, what the app said. */
    val score: Double,
    /** null = not yet answered. */
    val happened: Boolean?,
)

object PredictionStore {
    private const val KEY = "predictions.json"

    fun all(context: Context): List<Prediction> {
        val raw = ProfileStore.prefs(context).getString(KEY, null) ?: return emptyList()
        return runCatching {
            val arr = JSONArray(raw)
            (0 until arr.length()).map { i ->
                val o = arr.getJSONObject(i)
                Prediction(
                    life = o.getString("life"),
                    start = o.getLong("start"),
                    end = o.getLong("end"),
                    score = o.getDouble("score"),
                    happened = if (o.has("happened") && !o.isNull("happened")) o.getBoolean("happened") else null,
                )
            }
        }.getOrDefault(emptyList())
    }

    private fun save(context: Context, list: List<Prediction>) {
        val arr = JSONArray()
        for (p in list) {
            val o = JSONObject().put("life", p.life).put("start", p.start).put("end", p.end).put("score", p.score)
            if (p.happened != null) o.put("happened", p.happened)
            arr.put(o)
        }
        ProfileStore.prefs(context).edit().putString(KEY, arr.toString()).apply()
    }

    /** Remembers a named window; a window already remembered keeps its answer. */
    fun remember(context: Context, life: String, start: Long, end: Long, score: Double) {
        val list = all(context).toMutableList()
        if (list.any { it.life == life && kotlin.math.abs(it.start - start) < 36_000_000L }) return
        list.add(Prediction(life, start, end, score, null))
        val trimmed = list.groupBy { it.life }.values.flatMap { it.sortedBy { p -> p.start }.takeLast(40) }
        save(context, trimmed)
    }

    fun answer(context: Context, life: String, start: Long, happened: Boolean?) {
        val list = all(context).map { if (it.life == life && it.start == start) it.copy(happened = happened) else it }
        save(context, list)
    }

    /** Windows that have passed and are still unanswered, oldest first. */
    fun pending(context: Context, life: String? = null, now: Long = System.currentTimeMillis()): List<Prediction> =
        all(context).filter { it.happened == null && it.end < now && (life == null || it.life == life) }.sortedBy { it.start }

    /** Answered windows as the engine's record inputs. */
    fun records(context: Context): List<RecordInput> =
        all(context).filter { it.happened != null }.map { RecordInput(it.life, it.score, it.happened == true) }
}
