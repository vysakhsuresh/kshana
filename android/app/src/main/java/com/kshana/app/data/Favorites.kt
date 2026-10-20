package com.kshana.app.data

import android.content.Context
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateMapOf

/**
 * The pages a person keeps close: the ones they starred, in the order they starred them,
 * and a quiet count of how often each page under More is opened so the Favorites tab can
 * also offer the ones they keep coming back to. Kept in the profile store, so the backup
 * carries them.
 */
object Favorites {
    private const val K_PINNED = "fav.pinned"
    private const val K_OPENS = "fav.opens"

    /** Starred routes, oldest first. Observable from Compose. */
    val pinned = mutableStateListOf<String>()
    /** Route → times opened from More or Favorites. */
    val opens = mutableStateMapOf<String, Int>()
    private var loaded = false

    fun load(context: Context) {
        if (loaded) return
        loaded = true
        val p = ProfileStore.prefs(context)
        pinned.clear()
        pinned.addAll(p.getString(K_PINNED, "").orEmpty().split(',').filter { it.isNotBlank() })
        opens.clear()
        for (part in p.getString(K_OPENS, "").orEmpty().split(',')) {
            val kv = part.split('=')
            if (kv.size == 2) kv[1].toIntOrNull()?.let { opens[kv[0]] = it }
        }
    }

    /** Forget the cached copy (after a restore replaced the store). */
    fun reload(context: Context) { loaded = false; load(context) }

    fun isPinned(route: String) = route in pinned

    fun toggle(context: Context, route: String) {
        load(context)
        if (route in pinned) pinned.remove(route) else pinned.add(route)
        save(context)
    }

    fun move(context: Context, route: String, by: Int) {
        val i = pinned.indexOf(route)
        val j = i + by
        if (i < 0 || j !in pinned.indices) return
        pinned.removeAt(i)
        pinned.add(j, route)
        save(context)
    }

    fun opened(context: Context, route: String) {
        load(context)
        opens[route] = (opens[route] ?: 0) + 1
        ProfileStore.prefs(context).edit()
            .putString(K_OPENS, opens.entries.joinToString(",") { "${it.key}=${it.value}" }).apply()
    }

    private fun save(context: Context) {
        ProfileStore.prefs(context).edit().putString(K_PINNED, pinned.joinToString(",")).apply()
    }
}
