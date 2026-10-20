package com.kshana.app.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZoneOffset

class CityClocksTest {
    private fun offsetAt(id: String, local: LocalDateTime): ZoneOffset =
        CityClocks.zone(id, local)!!.rules.getOffset(local)

    @Test
    fun bombayIsBombayTimeUntil1955ThenIst() {
        assertEquals(ZoneOffset.ofHoursMinutes(4, 51), offsetAt(CityClocks.BOMBAY, LocalDateTime.of(1950, 6, 1, 10, 0)))
        assertEquals(ZoneOffset.ofHoursMinutes(4, 51), offsetAt(CityClocks.BOMBAY, LocalDateTime.of(1943, 6, 1, 10, 0)))
        assertEquals(ZoneOffset.ofHoursMinutes(4, 51), offsetAt(CityClocks.BOMBAY, LocalDateTime.of(1954, 12, 31, 23, 59)))
        assertEquals(ZoneOffset.ofHoursMinutes(5, 30), offsetAt(CityClocks.BOMBAY, LocalDateTime.of(1955, 6, 1, 10, 0)))
        assertEquals(ZoneOffset.ofHoursMinutes(5, 30), offsetAt(CityClocks.BOMBAY, LocalDateTime.of(1990, 10, 29, 23, 6)))
        // A 1950 Bombay birth at 10:00 city time is 05:09 UTC; read as IST it would be 04:30.
        val local = LocalDateTime.of(1950, 6, 1, 10, 0)
        assertEquals("1950-06-01T05:09:00Z", local.atZone(CityClocks.zone(CityClocks.BOMBAY, local)!!).toInstant().toString())
        // Present-day use of the same id is plain IST.
        assertEquals(ZoneId.of("Asia/Kolkata"), CityClocks.of(CityClocks.BOMBAY))
        assertEquals(ZoneId.of("Europe/London"), CityClocks.of("Europe/London"))
    }

    @Test
    fun calcuttaIsCalcuttaTimeUntil1948ThenIst() {
        assertEquals(ZoneOffset.ofHoursMinutesSeconds(5, 53, 20), offsetAt(CityClocks.CALCUTTA, LocalDateTime.of(1947, 8, 15, 0, 0)))
        assertEquals(ZoneOffset.ofHoursMinutes(5, 30), offsetAt(CityClocks.CALCUTTA, LocalDateTime.of(1948, 3, 1, 0, 0)))
        assertNull(CityClocks.zone("Not/AZone", LocalDateTime.of(1947, 1, 1, 0, 0)))
    }

    @Test
    fun placesInGreaterBombayAndCalcuttaGetTheCityClock() {
        assertEquals(CityClocks.BOMBAY, CityClocks.defaultZone("Asia/Kolkata", 19.12, 72.85, "Mumbai Suburban, Maharashtra"))
        assertEquals(CityClocks.BOMBAY, CityClocks.defaultZone("Asia/Kolkata", 18.93, 72.83, "Mumbai, Maharashtra"))
        assertEquals(CityClocks.CALCUTTA, CityClocks.defaultZone("Asia/Kolkata", 22.57, 88.36, "Kolkata, West Bengal"))
        assertEquals("Asia/Kolkata", CityClocks.defaultZone("Asia/Kolkata", 19.20, 72.97, "Thane, Maharashtra"))
        assertEquals("Asia/Kolkata", CityClocks.defaultZone("Asia/Kolkata", 22.59, 88.31, "Howrah, West Bengal"))
        // Online results carry no district: the box decides.
        assertEquals(CityClocks.BOMBAY, CityClocks.defaultZone("Asia/Kolkata", 19.076, 72.878))
        assertEquals("Asia/Kolkata", CityClocks.defaultZone("Asia/Kolkata", 19.20, 72.97))
        assertEquals("Asia/Kolkata", CityClocks.defaultZone("Asia/Kolkata", 22.59, 88.31))
        assertEquals("Asia/Karachi", CityClocks.defaultZone("Asia/Karachi", 19.076, 72.878))
    }

    @Test
    fun localMeanTimeIsLongitudeOverFifteen() {
        assertEquals("+05:05:04", CityClocks.localMeanTime(76.2673)) // Kochi: 76.2673 × 240 s = 18304 s
        assertEquals("-00:00:31", CityClocks.localMeanTime(-0.1278)) // London
        assertEquals("Z", CityClocks.localMeanTime(0.0))
    }

    @Test
    fun oldClockCard() {
        // 1950 Mumbai birth, default zone: reads in Bombay Time, offers IST.
        val a = OldClockCard.of(1950, 19.07, 72.88, CityClocks.BOMBAY, CityClocks.BOMBAY)!!
        assertTrue(a.lines[0].startsWith("Read in Bombay Time (UTC+04:51)"))
        assertEquals(listOf("Asia/Kolkata"), a.actions.map { it.second })
        // The person chose IST: offered Bombay Time back.
        val b = OldClockCard.of(1950, 19.07, 72.88, "Asia/Kolkata", CityClocks.BOMBAY)!!
        assertEquals(listOf(CityClocks.BOMBAY), b.actions.map { it.second })
        assertTrue(b.lines[0].contains("39 minutes behind Indian Standard Time"))
        // The switch year is flagged.
        assertEquals(2, OldClockCard.of(1955, 19.07, 72.88, CityClocks.BOMBAY, CityClocks.BOMBAY)!!.lines.size)
        // After the switch nothing to say; in Kochi after 1906 nothing either.
        assertNull(OldClockCard.of(1956, 19.07, 72.88, CityClocks.BOMBAY, CityClocks.BOMBAY))
        assertNull(OldClockCard.of(1990, 9.93, 76.27, "Asia/Kolkata", "Asia/Kolkata"))
        // Kochi 1890: local sun time offered.
        val c = OldClockCard.of(1890, 9.93, 76.27, "Asia/Kolkata", "Asia/Kolkata")!!
        assertEquals(listOf("+05:05:05"), c.actions.map { it.second })
        val d = OldClockCard.of(1890, 9.93, 76.27, "+05:05:05", "Asia/Kolkata")!!
        assertEquals(listOf("Asia/Kolkata"), d.actions.map { it.second })
        // Calcutta 23 min 20 s ahead.
        val e = OldClockCard.of(1940, 22.57, 88.36, "Asia/Kolkata", CityClocks.CALCUTTA)!!
        assertTrue(e.lines[0].contains("23 minutes 20 seconds ahead of Indian Standard Time"))
    }
}
