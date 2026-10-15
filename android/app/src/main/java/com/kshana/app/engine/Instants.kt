package com.kshana.app.engine

import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.UtcFields
import uniffi.kshana_ffi.instantFromUtc
import java.time.Instant
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.ZonedDateTime

/** Bridge between java.time and the engine's exact instants. */
object Instants {
    /** Now, to the nanosecond the device clock offers, as a TAI instant. */
    fun now(): ExactInstant = fromInstant(Instant.now())

    fun fromInstant(i: Instant): ExactInstant {
        val z = i.atZone(ZoneOffset.UTC)
        return instantFromUtc(
            UtcFields(
                year = z.year.toLong(),
                month = z.monthValue.toUInt(),
                day = z.dayOfMonth.toUInt(),
                hour = z.hour.toUInt(),
                minute = z.minute.toUInt(),
                second = z.second.toUInt(),
                nanos = z.nano.toUInt(),
            )
        )
    }

    /** Local wall clock for an exact instant (leap seconds render as 23:59:59.999…). */
    fun toLocal(e: ExactInstant, zone: ZoneId = ZoneId.systemDefault()): ZonedDateTime {
        val u = uniffi.kshana_ffi.utcFromInstant(e)
        val second = minOf(u.second.toInt(), 59)
        return ZonedDateTime.of(
            u.year.toInt(), u.month.toInt(), u.day.toInt(), u.hour.toInt(), u.minute.toInt(), second, u.nanos.toInt(), ZoneOffset.UTC
        ).withZoneSameInstant(zone)
    }
}
