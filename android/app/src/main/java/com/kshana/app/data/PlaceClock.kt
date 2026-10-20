package com.kshana.app.data

/** The place's clock offset from UTC right now, in minutes, for the calendar's civil day. */
object PlaceClock {
    fun offsetMinutes(place: Place, at: java.time.Instant = java.time.Instant.now()): Int =
        runCatching { java.time.ZoneId.of(place.zoneId).rules.getOffset(at).totalSeconds / 60 }
            .getOrElse { java.time.ZoneId.systemDefault().rules.getOffset(at).totalSeconds / 60 }
}
