package com.kshana.app.data

import android.content.Context

/**
 * The chart pages (wheels, periods, houses, the technical settings) are for the few who
 * read charts. Off by default: the person sees predictions in plain words only.
 */
object Expert {
    private const val KEY = "expert.on"

    fun on(context: Context): Boolean = ProfileStore.prefs(context).getBoolean(KEY, false)

    fun set(context: Context, on: Boolean) {
        ProfileStore.prefs(context).edit().putBoolean(KEY, on).apply()
    }
}
