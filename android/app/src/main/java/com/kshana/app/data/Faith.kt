package com.kshana.app.data

import android.content.Context

/**
 * Which remedy branches to show beyond the deeds everyone can do. "all" shows every
 * branch; "faith" adds prayer in one's own faith but not the Hindu poojas; "hindu" puts
 * the Hindu tradition first after the deeds; "deeds" shows deeds, giving, habits,
 * nature and quieting the mind only.
 */
object Faith {
    private const val KEY = "faith"
    val OPTIONS = listOf(
        "all" to "SHOW EVERYTHING",
        "hindu" to "HINDU TRADITION",
        "faith" to "MY OWN FAITH",
        "deeds" to "DEEDS ONLY",
    )

    fun get(context: Context): String = ProfileStore.prefs(context).getString(KEY, "all") ?: "all"
    fun set(context: Context, v: String) = ProfileStore.prefs(context).edit().putString(KEY, v).apply()

    /** Whether a branch (by key) is shown under the chosen preference. */
    fun shows(pref: String, branch: String): Boolean = when (pref) {
        "deeds" -> branch in setOf("deeds", "giving", "habits", "nature", "mind")
        "faith" -> branch != "tradition"
        else -> true
    }
}
