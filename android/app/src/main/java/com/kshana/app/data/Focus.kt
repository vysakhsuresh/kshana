package com.kshana.app.data

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/**
 * The parts of life that matter to this person. Summaries (the day, week, month and
 * year, the letter, what weighs most, the future) speak only of these; the others are
 * still read and shown below them, never in the headline. Someone without children is
 * not told about children every morning.
 */
object Focus {
    private const val KEY = "focus.lives"
    val ALL = listOf("Love", "Career", "Money", "Health", "Family", "Study", "Home", "Travel", "Children")
    private val DEFAULT = listOf("Love", "Career", "Money", "Health", "Family", "Home", "Travel")

    var lives by mutableStateOf(DEFAULT)
        private set

    fun load(context: Context) {
        val raw = ProfileStore.prefs(context).getString(KEY, null)
        lives = raw?.split(',')?.filter { it in ALL }?.takeIf { it.isNotEmpty() } ?: DEFAULT
    }

    fun toggle(context: Context, life: String) {
        val next = if (life in lives) lives - life else lives + life
        if (next.isEmpty()) return
        lives = ALL.filter { it in next }
        ProfileStore.prefs(context).edit().putString(KEY, lives.joinToString(",")).apply()
    }

    fun has(life: String) = life in lives
}
