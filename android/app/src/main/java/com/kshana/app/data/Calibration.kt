package com.kshana.app.data

import android.content.Context
import android.content.SharedPreferences

/**
 * The user's own record of whether a rule's reading came true, kept per rule and per
 * window (life area + antardasha start), so one answer counts once. Only windows a rule
 * marked as supporting are ever recorded, so "right" simply means the user said it happened.
 * Everything stays on the phone.
 */
object CalibrationStore {
    private const val PREFS = "kshana.calibration"
    private const val PREFIX = "cal:"

    private fun prefs(context: Context): SharedPreferences =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private fun key(ruleId: String, key: String): String = "$PREFIX$ruleId:$key"

    /** Records one answer for a rule and a window; a later answer for the same window replaces it. */
    fun record(context: Context, ruleId: String, key: String, happened: Boolean) {
        prefs(context).edit().putBoolean(key(ruleId, key), happened).apply()
    }

    /** Forgets the answer for a rule and a window (the user cleared both chips). */
    fun clear(context: Context, ruleId: String, key: String) {
        prefs(context).edit().remove(key(ruleId, key)).apply()
    }

    /** (right, total) for one rule: right = answers of "it happened", total = all answers. */
    fun stats(context: Context, ruleId: String): Pair<Int, Int> =
        allStats(context)[ruleId] ?: Pair(0, 0)

    /** (right, total) for every rule that has at least one answer. */
    fun allStats(context: Context): Map<String, Pair<Int, Int>> {
        val out = HashMap<String, Pair<Int, Int>>()
        for ((k, v) in prefs(context).all) {
            if (!k.startsWith(PREFIX) || v !is Boolean) continue
            // "cal:<ruleId>:<area>:<antarStart>": the rule id is the segment after the prefix.
            val rest = k.removePrefix(PREFIX)
            val cut = rest.indexOf(':')
            if (cut <= 0) continue
            val rule = rest.substring(0, cut)
            val (right, total) = out[rule] ?: Pair(0, 0)
            out[rule] = Pair(right + (if (v) 1 else 0), total + 1)
        }
        return out
    }

    /** "N answers across M rules". */
    fun summary(context: Context): String {
        val all = allStats(context)
        val answers = all.values.sumOf { it.second }
        val rules = all.size
        return "$answers ${if (answers == 1) "answer" else "answers"} across $rules ${if (rules == 1) "rule" else "rules"}"
    }
}
