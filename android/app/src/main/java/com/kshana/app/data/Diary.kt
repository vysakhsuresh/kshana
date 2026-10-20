package com.kshana.app.data

import android.content.Context
import org.json.JSONObject
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.SnapshotRecord

/**
 * One note and the sky it was written under. [atMillis] is the device clock at the press,
 * for sorting and local display; [taiSeconds] and [nanos] are the same press as the engine's
 * exact instant, so the note can be placed between the sky's own transitions later; [snapshot]
 * is the compact line of what was running then (see [DiaryStore.snapshotLine]).
 */
data class DiaryEntry(
    val atMillis: Long,
    val taiSeconds: Long,
    val nanos: UInt,
    val text: String,
    val snapshot: String,
) {
    fun instant(): ExactInstant = ExactInstant(taiSeconds, nanos)

    /** A stable identity for list keys and removal: one note per exact instant. */
    val key: String get() = "$taiSeconds.$nanos"
}

/**
 * The sky diary: the notes a person writes, one JSON object per line under the key
 * `diary.<profileName>` in the shared preferences file, newest first when read. Lines that no
 * longer parse are skipped rather than failing the whole list. Everything stays on the phone.
 */
object DiaryStore {
    private const val PREFIX = "diary."
    private const val NAKSHATRA_SPAN = 360.0 / 27.0

    private val SIGN_NAMES = listOf(
        "Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo",
        "Libra", "Scorpio", "Sagittarius", "Capricorn", "Aquarius", "Pisces",
    )
    private val NAKSHATRA_NAMES = listOf(
        "Ashwini", "Bharani", "Krittika", "Rohini", "Mrigashira", "Ardra", "Punarvasu", "Pushya", "Ashlesha",
        "Magha", "Purva Phalguni", "Uttara Phalguni", "Hasta", "Chitra", "Swati", "Vishakha", "Anuradha", "Jyeshtha",
        "Mula", "Purva Ashadha", "Uttara Ashadha", "Shravana", "Dhanishta", "Shatabhisha", "Purva Bhadrapada",
        "Uttara Bhadrapada", "Revati",
    )
    private val TITHI_NAMES = listOf(
        "Pratipada", "Dwitiya", "Tritiya", "Chaturthi", "Panchami", "Shashthi", "Saptami", "Ashtami",
        "Navami", "Dashami", "Ekadashi", "Dwadashi", "Trayodashi", "Chaturdashi",
    )

    private fun key(profileName: String): String = "$PREFIX$profileName"

    private fun norm360(x: Double): Double = (x % 360.0 + 360.0) % 360.0
    private fun signName(sidereal: Double): String = SIGN_NAMES[(norm360(sidereal) / 30.0).toInt() % 12]
    private fun nakshatraName(sidereal: Double): String = NAKSHATRA_NAMES[(norm360(sidereal) / NAKSHATRA_SPAN).toInt() % 27]

    /** "Shukla Dashami", "Purnima", "Krishna Tritiya", "Amavasya" for the engine's 1–30 numbering. */
    fun tithiName(number: Int): String = when {
        number == 15 -> "Purnima"
        number == 30 -> "Amavasya"
        number in 1..14 -> "Shukla ${TITHI_NAMES[number - 1]}"
        number in 16..29 -> "Krishna ${TITHI_NAMES[number - 16]}"
        else -> "tithi $number"
    }

    /**
     * The sky at one instant on one line: the dasha chain to three levels, the Moon's sign and
     * star, the rising sign, the tithi and the hora. "Saturn › Venus › Mercury · Moon in Taurus,
     * Rohini · Gemini rising · Shukla Dashami · Venus hora".
     */
    fun snapshotLine(s: SnapshotRecord): String {
        val parts = ArrayList<String>(5)
        val chain = s.dashaLords.take(3)
        if (chain.isNotEmpty()) parts += chain.joinToString(" › ")
        s.grahas.firstOrNull { it.name == "Moon" }?.let { parts += "Moon in ${signName(it.sidereal)}, ${nakshatraName(it.sidereal)}" }
        s.grahas.firstOrNull { it.name == "Lagna" }?.let { parts += "${signName(it.sidereal)} rising" }
        parts += tithiName(s.tithi.toInt())
        s.horaLord?.let { parts += "$it hora" }
        return parts.joinToString(" · ")
    }

    private fun newestFirst(list: List<DiaryEntry>): List<DiaryEntry> =
        list.sortedWith(compareByDescending<DiaryEntry> { it.atMillis }.thenByDescending { it.taiSeconds }.thenByDescending { it.nanos.toLong() })

    private fun toJson(e: DiaryEntry): String = JSONObject()
        .put("at", e.atMillis)
        .put("tai", e.taiSeconds)
        .put("nanos", e.nanos.toLong())
        .put("text", e.text)
        .put("snapshot", e.snapshot)
        .toString()

    private fun fromJson(line: String): DiaryEntry? = runCatching {
        val o = JSONObject(line)
        DiaryEntry(
            atMillis = o.getLong("at"),
            taiSeconds = o.getLong("tai"),
            nanos = o.optLong("nanos", 0L).coerceIn(0L, 999_999_999L).toUInt(),
            text = o.optString("text", ""),
            snapshot = o.optString("snapshot", ""),
        )
    }.getOrNull()

    /** Every note for the profile, newest first; unreadable lines are dropped. */
    fun entries(context: Context, profileName: String): List<DiaryEntry> {
        val raw = ProfileStore.prefs(context).getString(key(profileName), null) ?: return emptyList()
        return newestFirst(raw.split('\n').filter { it.isNotBlank() }.mapNotNull(::fromJson))
    }

    /** Replaces the whole list. */
    fun save(context: Context, profileName: String, entries: List<DiaryEntry>) {
        val lines = newestFirst(entries).joinToString("\n") { toJson(it) }
        ProfileStore.prefs(context).edit().putString(key(profileName), lines).apply()
    }

    /** Keeps one note and returns the new list, newest first. */
    fun add(context: Context, profileName: String, entry: DiaryEntry): List<DiaryEntry> {
        val list = newestFirst(entries(context, profileName).filter { it.key != entry.key } + entry)
        save(context, profileName, list)
        return list
    }

    /** Removes the note with [entry]'s instant and returns the new list; an unknown note changes nothing. */
    fun remove(context: Context, profileName: String, entry: DiaryEntry): List<DiaryEntry> {
        val current = entries(context, profileName)
        val list = current.filter { it.key != entry.key }
        if (list.size != current.size) save(context, profileName, list)
        return list
    }
}
