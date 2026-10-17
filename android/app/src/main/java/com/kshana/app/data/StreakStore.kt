package com.kshana.app.data

import android.content.Context
import java.time.LocalDate

/**
 * The remedy streak: each committed remedy can be ticked once a day; the streak is the
 * run of consecutive days ending today (or yesterday, so a morning tick keeps it).
 */
object StreakStore {
    private fun key(planet: String, title: String) = "streak:$planet:$title"

    fun days(context: Context, planet: String, title: String): Set<LocalDate> =
        (ProfileStore.prefs(context).getStringSet(key(planet, title), emptySet()) ?: emptySet())
            .mapNotNull { runCatching { LocalDate.parse(it) }.getOrNull() }.toSet()

    fun tickedToday(context: Context, planet: String, title: String): Boolean = LocalDate.now() in days(context, planet, title)

    fun toggleToday(context: Context, planet: String, title: String) {
        val today = LocalDate.now()
        val d = days(context, planet, title).toMutableSet()
        if (today in d) d.remove(today) else d.add(today)
        // Keep a year at most.
        val cut = today.minusDays(400)
        ProfileStore.prefs(context).edit().putStringSet(key(planet, title), d.filter { it.isAfter(cut) }.map { it.toString() }.toSet()).apply()
    }

    /** Consecutive days ending today or yesterday. */
    fun streak(context: Context, planet: String, title: String): Int {
        val d = days(context, planet, title)
        var day = LocalDate.now()
        if (day !in d) day = day.minusDays(1)
        var n = 0
        while (day in d) { n++; day = day.minusDays(1) }
        return n
    }

    /** The longest streak across every committed remedy of a planet. */
    fun best(context: Context, planet: String, titles: List<String>): Int = titles.maxOfOrNull { streak(context, planet, it) } ?: 0
}
