package com.kshana.app.data

import android.content.Context
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf

/**
 * Helps people find the pages they would never think to open: which pages they have
 * already seen, the date they began (for the first-week guide), and the pages they waved
 * away from the "Try this" card. Kept in the profile store, so the backup carries it.
 */
object Discover {
    private const val K_SEEN = "disc.seen"
    private const val K_SINCE = "disc.since"
    private const val K_SKIP = "disc.skip"

    /** One door: a page, why it is worth opening, and the button's words. */
    data class Door(val route: String, val title: String, val why: String, val go: String)

    /** The first week: one door a day, in this order. */
    val WEEK = listOf(
        Door("pastcheck", "Let it tell you your past", "A few dated stretches of your own life, read from your birth time alone. Say Yes, Close or No; every answer sharpens your birth time.", "READ MY PAST"),
        Door("reading", "Your reading", "Tell it what is on your mind (work, money, love, health) and it reads that part of your life, with dates.", "OPEN YOUR READING"),
        Door("future", "The years ahead", "Every opening and every stretch to tend, in order, with the month and year it begins.", "SEE WHAT IS COMING"),
        Door("parihara", "What helps", "For whatever weighs on you now: deeds to do, things to give, and a prayer if you wish. Each one says why it helps.", "SEE WHAT HELPS"),
        Door("together", "The two of you", "Add your partner or family. See how you match, how things stand between you today, and what each can do for the other.", "OPEN US"),
        Door("wall", "What you are waiting for", "Every date that matters to you, counting down, with what each one holds.", "OPEN THE WALL"),
        Door("book", "The book of your life", "The years behind and ahead as chapters. Check the past ones against what happened.", "OPEN THE BOOK"),
    )

    /** After the first week: pages offered one at a time, until each has been opened. */
    private val LATER = listOf(
        Door("familyproof", "Family proof", "Add your family, and see where their charts and yours agree about the same months.", "READ US TOGETHER"),
        Door("scoreboard", "Your score", "Predictions are sealed before they come. Here is how many came true.", "SEE MY SCORE"),
        Door("journal", "Your journal", "Write down what happened. Over time it shows how often the app was right for you.", "WRITE THE FIRST LINE"),
        Door("calendar", "Your week's appointments", "Your calendar, checked hour by hour; a weak slot gets a better one beside it.", "CHECK MY WEEK"),
        Door("plan", "Plan a day out", "Say what you are going for; it picks the best hour and says what to keep in mind.", "PLAN A DAY"),
        Door("familydates", "A date for all of us", "A wedding, a house-warming, a trip: the days that suit everyone at once.", "FIND A DATE"),
        Door("letter", "The month's letter", "One calm page for the thirty days ahead.", "READ THE LETTER"),
        Door("hindsight", "Look back at a day", "Any day of your life, read as it was. A good way to see if the reading rings true.", "PICK A DAY"),

        Door("guide", "The guide", "How to use Kshana, screen by screen, with tips.", "OPEN THE GUIDE"),
    )

    /** Seen routes, observable so badges update as soon as a page is opened. */
    val seen = mutableStateListOf<String>()
    private val skipped = mutableStateListOf<String>()
    private val since = mutableStateOf(0L)
    private var loaded = false

    fun load(context: Context) {
        if (loaded) return
        loaded = true
        val p = ProfileStore.prefs(context)
        seen.clear(); seen.addAll(p.getString(K_SEEN, "").orEmpty().split(',').filter { it.isNotBlank() })
        // Someone who used the app before this store existed: count what they opened from More.
        if (!p.contains(K_SEEN)) {
            Favorites.load(context)
            seen.addAll(Favorites.opens.keys.filter { it !in seen })
        }
        skipped.clear(); skipped.addAll(p.getString(K_SKIP, "").orEmpty().split(',').filter { it.isNotBlank() })
        var s = p.getLong(K_SINCE, 0L)
        if (s == 0L) {
            s = java.time.LocalDate.now().toEpochDay()
            p.edit().putLong(K_SINCE, s).apply()
        }
        since.value = s
    }

    /** Forget the cached copy (after a restore replaced the store). */
    fun reload(context: Context) { loaded = false; load(context) }

    /** Called on every screen change. The route pattern is stripped of its arguments. */
    fun visit(context: Context, route: String) {
        load(context)
        val r = route.substringBefore('/')
        if (r in seen) return
        seen.add(r)
        ProfileStore.prefs(context).edit().putString(K_SEEN, seen.joinToString(",")).apply()
    }

    fun isNew(route: String) = route !in seen

    fun skip(context: Context, route: String) {
        load(context)
        if (route !in skipped) skipped.add(route)
        ProfileStore.prefs(context).edit().putString(K_SKIP, skipped.joinToString(",")).apply()
    }

    /** Day of the first week, 1 to 7; null once the week is over. */
    fun weekDay(): Int? {
        val d = (java.time.LocalDate.now().toEpochDay() - since.value).toInt() + 1
        return d.takeIf { it in 1..7 }
    }

    /**
     * The one card for Now. In the first week, that day's door (or the next one not yet
     * opened); after it, one page not yet opened. A [testing] stretch puts What helps first.
     */
    fun pick(testing: Boolean): Pair<Door, Int?>? {
        val day = weekDay()
        if (day != null) {
            val today = WEEK.drop(day - 1).firstOrNull { it.route !in seen } ?: WEEK.firstOrNull { it.route !in seen }
            if (today != null) return today to day
        }
        val pool = (if (testing) WEEK.sortedByDescending { it.route == "parihara" } else WEEK) + LATER
        return pool.firstOrNull { it.route !in seen && it.route !in skipped }?.let { it to null }
    }
}
