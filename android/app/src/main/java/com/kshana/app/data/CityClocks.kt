package com.kshana.app.data

import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZoneOffset
import kotlin.math.roundToInt

/**
 * The old city clocks of India, as time zones the rest of the app can use like any other.
 *
 * The time zone database files all of India under Asia/Kolkata: Madras railway time
 * (+05:21:10) until 1906, then Indian Standard Time (+05:30). But two cities kept their own
 * civil clock long after 1906, and a birth registered in either city before the switch was
 * usually written in that clock:
 *
 * - **Bombay Time, UTC+04:51**, kept by the city until 1955 (39 minutes behind IST).
 * - **Calcutta Time, UTC+05:53:20**, kept by the city until 1948 (23 min 20 s ahead of IST).
 *
 * Sources: tzdb `asia` file, India section (P. Eggert, citing Reed & Low, *The Indian Year
 * Book 1936-37*, pp. 27-8: railways and telegraphs went to +05:30 on 1906-01-01 while "some
 * municipalities retained their former time"; Calcutta government time +05:53:20 per Shanks);
 * *Science* 23 (1906-04-06) on the 1906 change, which gives Calcutta Time as 23 m 20 s ahead
 * of IST and 1 h 2 m 20 s ahead of Bombay Time (so Bombay = +04:51:00); Encyclopaedia
 * Britannica, "Indian time zone" ("Calcutta would adopt IST in 1948, and Bombay would follow
 * suit only in 1955"); Wikipedia "Bombay Time", "Calcutta Time". No source found gives the
 * day of either switch, only the year, so the switch is placed at the start of that year
 * ([Clock.switchYear]) and a birth inside the switch year is flagged for the person to check.
 *
 * Madras kept no separate clock after 1906: Madras Time (+05:21:10) was the railway standard
 * before IST and is already what tzdb gives for Asia/Kolkata before 1906. Wartime: tzdb
 * (after Shanks, which it calls dubious) has India at +06:30 for 1941-10 … 1942-05-15 and
 * 1942-09 … 1945-10-15; nothing found says whether the city clocks were also advanced, so the
 * city clocks here stay at their fixed offset throughout. After the switch each city clock is
 * Asia/Kolkata exactly.
 *
 * The ids ("Kshana/Bombay", "Kshana/Calcutta") are not java.time ids (Android does not let an
 * app register zone rules), so stored ids go through [of] (the zone of the present day, IST
 * for a city clock) or [zone] (the zone in force at a given local time, the city's fixed
 * offset before its switch) instead of `ZoneId.of`.
 */
object CityClocks {
    const val BOMBAY = "Kshana/Bombay"
    const val CALCUTTA = "Kshana/Calcutta"
    private const val IST_ZONE = "Asia/Kolkata"

    /** A city's own clock: its offset, the year it went over to IST, and where it applied. */
    data class Clock(
        val id: String,
        val city: String,
        val offset: ZoneOffset,
        /** The year the city clock gave way to IST; the switch is taken at 1 January. */
        val switchYear: Int,
        /** Offline-gazetteer districts ("District, State", as in assets/regions.tsv) that kept the city clock. */
        val districts: Set<String>,
        /** A conservative box (lat, lon) around the city for places found without a district. */
        val box: DoubleArray,
        /** City centre, for the "near the city" hint. */
        val lat: Double,
        val lon: Double,
    ) {
        fun gapText(): String {
            val d = (IST.totalSeconds - offset.totalSeconds)
            val m = kotlin.math.abs(d) / 60
            val s = kotlin.math.abs(d) % 60
            val amount = if (s == 0) "$m minutes" else "$m minutes $s seconds"
            return if (d > 0) "$amount behind Indian Standard Time" else "$amount ahead of Indian Standard Time"
        }
        fun offsetText(): String = "UTC" + offset.id
    }

    private val IST: ZoneOffset = ZoneOffset.ofHoursMinutes(5, 30)

    /**
     * Greater Bombay: the two GeoNames districts that make up today's Mumbai (the island city
     * and the suburbs, Bombay Suburban District from 1950), the area administered as Bombay
     * when the city clock ended. Thane and Navi Mumbai lie outside. Calcutta: the Kolkata
     * district, the Calcutta Corporation area; Howrah across the river was a railway town on
     * railway time, so it is only offered the clock, not given it. The boxes, for places
     * found online where no district is known, are kept inside the same ground: Greater
     * Bombay west of Thane (72.955° E), and the Calcutta core east of the Hooghly (88.34° E)
     * so that Howrah is left out.
     */
    val all: List<Clock> = listOf(
        Clock(
            BOMBAY, "Bombay", ZoneOffset.ofHoursMinutes(4, 51), 1955,
            setOf("Mumbai, Maharashtra", "Mumbai Suburban, Maharashtra"),
            doubleArrayOf(18.89, 19.27, 72.77, 72.955), 18.94, 72.83,
        ),
        Clock(
            CALCUTTA, "Calcutta", ZoneOffset.ofHoursMinutesSeconds(5, 53, 20), 1948,
            setOf("Kolkata, West Bengal"),
            doubleArrayOf(22.50, 22.62, 88.34, 88.41), 22.57, 88.36,
        ),
    )

    fun byId(id: String): Clock? = all.firstOrNull { it.id == id.trim() }

    /** First local wall time on IST: the city clock ran up to midnight starting [Clock.switchYear]. */
    private fun switchAt(c: Clock): LocalDateTime = LocalDateTime.of(c.switchYear, 1, 1, 0, 0)

    /**
     * `ZoneId.of` that also knows the city-clock ids, which it maps to Asia/Kolkata: the right
     * zone for any date after the switch (today, transits, events). Throws DateTimeException
     * for an unknown id, as `ZoneId.of` does.
     */
    fun of(id: String): ZoneId = if (byId(id) != null) ZoneId.of(IST_ZONE) else ZoneId.of(id.trim())

    /**
     * The zone in force at the local wall time [local] for a stored id: a city-clock id is the
     * city's fixed offset before its switch and Asia/Kolkata from then on; any other id is
     * `ZoneId.of(id)`. Null when the id is unknown.
     */
    fun zone(id: String, local: LocalDateTime?): ZoneId? {
        val c = byId(id) ?: return runCatching { ZoneId.of(id.trim()) }.getOrNull()
        return if (local != null && local.isBefore(switchAt(c))) c.offset else ZoneId.of(IST_ZONE)
    }

    /** The city clock a gazetteer district kept, or null. */
    fun forDistrict(region: String): Clock? = all.firstOrNull { region.trim() in it.districts }

    /** The city clock whose box holds the point, or null. */
    fun forPoint(lat: Double, lon: Double): Clock? =
        all.firstOrNull { lat >= it.box[0] && lat <= it.box[1] && lon >= it.box[2] && lon <= it.box[3] }

    /**
     * The zone id a place should carry by default: the city clock for places in Greater Bombay
     * or Calcutta that tzdb puts on Asia/Kolkata, the tzdb zone otherwise. A city-clock id is
     * Asia/Kolkata for every date after the switch, so this changes nothing for later births.
     */
    fun defaultZone(tzdbZone: String, lat: Double, lon: Double, region: String? = null): String {
        if (tzdbZone != IST_ZONE) return tzdbZone
        val c = if (region != null) forDistrict(region) else forPoint(lat, lon)
        return c?.id ?: tzdbZone
    }

    /** A city clock within about 60 km of the point (for the hint), or null. */
    fun near(lat: Double, lon: Double): Clock? =
        all.firstOrNull { kotlin.math.abs(lat - it.lat) < 0.6 && kotlin.math.abs(lon - it.lon) < 0.6 }

    /**
     * Local mean time at a longitude, as a fixed offset id: longitude / 15 hours (east
     * positive), to the second. The clock of the sun's mean noon at that meridian, which is
     * what town clocks kept before railway and standard time.
     */
    fun localMeanTime(lon: Double): String =
        ZoneOffset.ofTotalSeconds((lon.coerceIn(-180.0, 180.0) * 240.0).roundToInt()).id
}

/**
 * What the birth screen's old-clock card says and offers, worked out without any UI so it
 * can be tested. Null when no card is needed (no city clock applies and the record is from
 * 1906 or later).
 *
 * @param zone the zone id now in the form; [placeZone] the zone the picked place carries.
 * Old profiles saved before the city-clock ids may hold the bare offsets "+04:51" or
 * "+05:53:20"; those count as the city clock.
 */
data class OldClockCard(val lines: List<String>, val actions: List<Pair<String, String>>) {
    companion object {
        fun of(year: Int, lat: Double, lon: Double, zone: String, placeZone: String): OldClockCard? {
            val z = zone.trim()
            val chosen = CityClocks.byId(z) ?: CityClocks.all.firstOrNull { it.offset.id == z }
            val clock = (chosen ?: CityClocks.near(lat, lon))?.takeIf { year <= it.switchYear }
            // India before IST (1906-01-01): only railways kept one clock (Madras time).
            val veryOld = year < 1906 && lat in 6.0..37.5 && lon in 68.0..98.0
            val lmt = CityClocks.localMeanTime(lon)
            val usingLmt = z == lmt
            if (clock == null && !veryOld && !usingLmt) return null
            val usingCity = clock != null && chosen == clock
            val pz = placeZone.trim()
            val placeIsFixed = pz.startsWith("+") || pz.startsWith("-") || pz == "Z"
            val standard = when {
                CityClocks.byId(pz) != null -> "Asia/Kolkata"
                placeIsFixed -> if (clock != null || lon in 68.0..98.0) "Asia/Kolkata" else null
                else -> pz
            }
            val usingStandard = standard != null && z == standard
            val standardName = if (standard == "Asia/Kolkata") "Indian Standard Time" else standard
            val lines = buildList {
                if (clock != null) {
                    add(
                        if (usingCity) "Read in ${clock.city} Time (${clock.offsetText()}), the city's own clock until ${clock.switchYear}."
                        else "${clock.city} kept its own clock until ${clock.switchYear}, ${clock.gapText()}. Home births and older registers were often written in it; if the record says IST, keep Indian Standard Time.",
                    )
                    if (year == clock.switchYear) {
                        add("The city moved to Indian Standard Time during ${clock.switchYear} and the day is not on record. If the birth came after the change, use Indian Standard Time.")
                    }
                }
                if (usingLmt) {
                    add("Read in local sun time here (UTC$lmt): the clock of the sun's own noon at this longitude.")
                } else if (veryOld) {
                    add("Before 1906 India had no single clock and many towns kept the sun's own time. If the record is that old and written in town time, local sun time here is UTC$lmt.")
                }
            }
            val actions = buildList {
                if (clock != null && !usingCity) add("USE ${clock.city.uppercase()} TIME ›" to clock.id)
                if (standard != null && standardName != null && !usingStandard) add("USE ${standardName.uppercase()} ›" to standard)
                if (veryOld && !usingLmt) add("USE LOCAL SUN TIME ›" to lmt)
            }
            return OldClockCard(lines, actions)
        }
    }
}
