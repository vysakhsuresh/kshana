package com.kshana.app.data

import android.content.Context
import uniffi.kshana_ffi.term
import java.util.concurrent.ConcurrentHashMap

/**
 * The language the app shows its regional terms in: `en` (the engine's own names), `ml`, `ta`,
 * `hi`, `te` or `kn`. The choice lives in the same SharedPreferences file as the profile, under
 * `ui.language`, and defaults to English. Terms come from the engine's lexicon through the FFI
 * `term(key, code)` and are cached in memory, so a screen can ask for a word on every
 * recomposition without crossing the boundary again.
 */
object LanguageStore {
    private const val K_LANG = "ui.language"

    /** English: every key passes through unchanged. */
    const val DEFAULT = "en"

    /** Every code the lexicon knows, in the order the chips show them. */
    val CODES = listOf("en", "ml", "ta", "hi", "te", "kn")

    private val cache = ConcurrentHashMap<String, String>()

    /** The chosen language code; `en` until the user picks one. */
    fun code(context: Context): String =
        ProfileStore.prefs(context).getString(K_LANG, DEFAULT)?.takeIf { it.isNotBlank() } ?: DEFAULT

    /** Store a language code (`ml`, `ta`, …); anything unknown falls back to English in the engine. */
    fun save(context: Context, code: String) {
        ProfileStore.prefs(context).edit().putString(K_LANG, code.trim().lowercase()).apply()
        cache.clear()
    }

    /**
     * The term [key] in the chosen language, native script. [key] is the engine's spelling
     * (`"Ashwini"`, `"Shukla Pratipada"`, `"Ravivara"`, `"Rahu kala"`) or an alias (`"Mesha"`,
     * `"Sunday"`, `"House 7"`). In English, or when the cell is not verified for the language, or
     * when the key is unknown, the key comes back unchanged, so the call is always safe to show.
     */
    fun t(context: Context, key: String): String {
        val code = code(context)
        if (code == DEFAULT || key.isBlank()) return key
        return cache.getOrPut("$code|$key") {
            runCatching { term(key, code) }.getOrDefault(key).ifBlank { key }
        }
    }
}

/** Shorthand for [LanguageStore.t]: `t(context, "Ashwini")` is `അശ്വതി` once Malayalam is chosen. */
fun t(context: Context, key: String): String = LanguageStore.t(context, key)
