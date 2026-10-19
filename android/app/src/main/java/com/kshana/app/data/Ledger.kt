package com.kshana.app.data

import android.content.Context
import uniffi.kshana_ffi.LedgerEventRecord
import uniffi.kshana_ffi.ledgerEventFromJson
import uniffi.kshana_ffi.ledgerEventToJson

/**
 * The life ledger: the dated events the user has logged for one profile, kept as one
 * JSON line per event (the engine's own format, so the Rust side reads it back) under
 * the key `ledger.<profileName>` in the shared preferences file. Lines that no longer
 * parse are skipped rather than failing the whole list. Events are kept in time order,
 * so an index into [events] is stable until the list changes. Everything stays on the phone.
 */
object LedgerStore {
    private const val PREFIX = "ledger."

    private fun key(profileName: String): String = "$PREFIX$profileName"

    private fun ordered(events: List<LedgerEventRecord>): List<LedgerEventRecord> =
        events.sortedWith(compareBy({ it.at.taiSeconds }, { it.at.nanos }))

    /** Every stored event for the profile, oldest first; unreadable lines are dropped. */
    fun events(context: Context, profileName: String): List<LedgerEventRecord> {
        val raw = ProfileStore.prefs(context).getString(key(profileName), null) ?: return emptyList()
        val parsed = raw.split('\n')
            .filter { it.isNotBlank() }
            .mapNotNull { line -> runCatching { ledgerEventFromJson(line) }.getOrNull() }
        return ordered(parsed)
    }

    /** Replaces the whole list; events the engine refuses to serialise are left out. */
    fun save(context: Context, profileName: String, events: List<LedgerEventRecord>) {
        val lines = ordered(events).mapNotNull { e -> runCatching { ledgerEventToJson(e) }.getOrNull() }
        ProfileStore.prefs(context).edit().putString(key(profileName), lines.joinToString("\n")).apply()
    }

    /** Appends one event and returns the new list, in time order. */
    fun add(context: Context, profileName: String, event: LedgerEventRecord): List<LedgerEventRecord> {
        val list = ordered(events(context, profileName) + event)
        save(context, profileName, list)
        return list
    }

    /** Removes the event at [index] of [events] and returns the new list; an index out of range changes nothing. */
    fun remove(context: Context, profileName: String, index: Int): List<LedgerEventRecord> {
        val current = events(context, profileName)
        if (index !in current.indices) return current
        val list = current.filterIndexed { i, _ -> i != index }
        save(context, profileName, list)
        return list
    }
}
