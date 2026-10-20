package com.kshana.app.data

import android.content.Context
import uniffi.kshana_ffi.SealedRecord
import uniffi.kshana_ffi.sealedForecastFromJson

/**
 * One stored forecast: the engine's JSON line exactly as it was written, and the
 * record read back from it. The line is the thing of record; `record.verified` says
 * whether its id still matches the claim.
 */
data class SealedEntry(val line: String, val record: SealedRecord)

/**
 * Sealed forecasts for one profile, kept as one JSON line per forecast (the engine's
 * own format, stored verbatim from `sealedForecastToJson`) under the key
 * `sealed.<profileName>` in the shared preferences file, the same way the ledger keeps
 * its events. Lines are only ever appended or replaced one at a time, never rewritten
 * from parsed records, so a line the engine cannot read is kept and skipped rather
 * than lost. Everything stays on the phone.
 */
object SealedStore {
    private const val PREFIX = "sealed."

    private fun key(profileName: String): String = "$PREFIX$profileName"

    /** Every stored line, in the order written. */
    fun lines(context: Context, profileName: String): List<String> {
        val raw = ProfileStore.prefs(context).getString(key(profileName), null) ?: return emptyList()
        return raw.split('\n').filter { it.isNotBlank() }
    }

    /** Every readable forecast with its line, newest sealed first; unreadable lines are skipped. */
    fun entries(context: Context, profileName: String): List<SealedEntry> =
        lines(context, profileName)
            .mapNotNull { line -> runCatching { sealedForecastFromJson(line) }.getOrNull()?.let { SealedEntry(line, it) } }
            .sortedWith(compareByDescending<SealedEntry> { it.record.createdAt.taiSeconds }.thenByDescending { it.record.createdAt.nanos })

    /** How many stored lines the engine could not read. */
    fun unreadable(context: Context, profileName: String): Int =
        lines(context, profileName).count { line -> runCatching { sealedForecastFromJson(line) }.isFailure }

    private fun write(context: Context, profileName: String, lines: List<String>) {
        ProfileStore.prefs(context).edit().putString(key(profileName), lines.joinToString("\n")).apply()
    }

    /** Appends one line as the engine wrote it and returns the new entries. */
    fun add(context: Context, profileName: String, line: String): List<SealedEntry> {
        write(context, profileName, lines(context, profileName) + line.trim())
        return entries(context, profileName)
    }

    /** Replaces one stored line by another (a resolution) and returns the new entries; an unknown line changes nothing. */
    fun replace(context: Context, profileName: String, oldLine: String, newLine: String): List<SealedEntry> {
        val current = lines(context, profileName)
        val index = current.indexOf(oldLine)
        if (index >= 0) {
            write(context, profileName, current.mapIndexed { i, l -> if (i == index) newLine.trim() else l })
        }
        return entries(context, profileName)
    }
}
