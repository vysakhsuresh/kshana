package com.kshana.app.system

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.core.app.NotificationCompat
import com.kshana.app.MainActivity
import com.kshana.app.R
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import kotlinx.coroutines.runBlocking
import uniffi.kshana_ffi.BriefingRecord
import java.time.Instant
import java.time.ZoneOffset
import java.time.ZonedDateTime
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.UtcFields

/**
 * The sunrise reading: one notification at the person's sunrise with the moment's name,
 * the day's reading and Saturn's line, computed on the phone when the alarm fires. The
 * alarm is set for the next sunrise found by the engine and re-armed after each firing
 * and after a reboot. Nothing leaves the phone.
 */
object Sunrise {
    private const val CHANNEL = "sunrise"
    private const val REQUEST = 4101
    private const val K_ON = "sunrise.on"
    private const val K_NEXT = "sunrise.next"

    fun enabled(context: Context): Boolean = ProfileStore.prefs(context).getBoolean(K_ON, false)

    fun setEnabled(context: Context, on: Boolean) {
        ProfileStore.prefs(context).edit().putBoolean(K_ON, on).apply()
        if (on) scheduleNext(context) else cancel(context)
    }

    fun nextMillis(context: Context): Long = ProfileStore.prefs(context).getLong(K_NEXT, 0L)

    private fun pending(context: Context): PendingIntent {
        val i = Intent(context, SunriseReceiver::class.java).setAction("com.kshana.app.SUNRISE")
        return PendingIntent.getBroadcast(context, REQUEST, i, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
    }

    fun cancel(context: Context) {
        (context.getSystemService(Context.ALARM_SERVICE) as AlarmManager).cancel(pending(context))
        ProfileStore.prefs(context).edit().remove(K_NEXT).apply()
    }

    /** Finds the next sunrise after now at the current place and arms the alarm for it. */
    fun scheduleNext(context: Context) {
        val profile = ProfileStore.load(context) ?: return
        val place = ProfileStore.currentLocation(context)
        val settings = ProfileStore.settings(context)
        Thread {
            try {
                val engine = runBlocking { EngineProvider.get(context) }
                val now = Instant.now()
                // The moment an hour from now carries today's sunrise and the next; take the first after now.
                var next: Instant? = null
                for (h in listOf(1L, 13L, 25L)) {
                    val m = engine.forecastMoment(
                        profile.birthInstant(), profile.latitude, profile.longitude,
                        place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa,
                        Instants.fromInstant(now.plusSeconds(h * 3600)),
                    )
                    val s = m.sunrise?.let { Instants.toLocal(it).toInstant() }
                    if (s != null && s.isAfter(now.plusSeconds(60))) { next = s; break }
                }
                val target = next ?: return@Thread
                val am = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
                am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, target.toEpochMilli(), pending(context))
                ProfileStore.prefs(context).edit().putLong(K_NEXT, target.toEpochMilli()).apply()
            } catch (_: Exception) {
            }
        }.start()
    }

    fun channel(context: Context) {
        if (Build.VERSION.SDK_INT >= 26) {
            val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            nm.createNotificationChannel(NotificationChannel(CHANNEL, "Sunrise reading", NotificationManager.IMPORTANCE_DEFAULT).apply {
                description = "One reading at your sunrise, computed on this phone."
            })
        }
    }

    /** Computes the briefing now and posts it. Returns the briefing for the widget. */
    fun post(context: Context): BriefingRecord? {
        val profile = ProfileStore.load(context) ?: return null
        val place = ProfileStore.currentLocation(context)
        val settings = ProfileStore.settings(context)
        return try {
            val engine = runBlocking { EngineProvider.get(context) }
            val b = engine.briefing(
                profile.birthInstant(), profile.latitude, profile.longitude,
                place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa,
                listOf("Love", "Career", "Money", "Health"), Instants.now(),
            )
            channel(context)
            val open = PendingIntent.getActivity(context, 0, Intent(context, MainActivity::class.java), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            val m = b.moment
            val top = m.lives.maxByOrNull { it.score }
            val text = "${m.title}. ${top?.headline ?: ""}".trim()
            val big = "${b.paragraphs.getOrNull(2) ?: text}\n\n${b.saturnWords}."
            val n = NotificationCompat.Builder(context, CHANNEL)
                .setSmallIcon(R.mipmap.ic_launcher)
                .setContentTitle("Sunrise · ${m.overall.toInt()}, ${m.band}")
                .setContentText(text)
                .setStyle(NotificationCompat.BigTextStyle().bigText(big))
                .setContentIntent(open)
                .setAutoCancel(true)
                .build()
            val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            if (Build.VERSION.SDK_INT < 33 || context.checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) == android.content.pm.PackageManager.PERMISSION_GRANTED) {
                nm.notify(REQUEST, n)
            }
            b
        } catch (_: Exception) {
            null
        }
    }
}

/** Fires at sunrise: posts the reading, refreshes the widget, arms the next sunrise. */
class SunriseReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val result = goAsync()
        Thread {
            try {
                if (Sunrise.enabled(context)) {
                    val b = Sunrise.post(context)
                    if (b != null) SadeSatiWidget.push(context, b)
                    Sunrise.scheduleNext(context)
                }
            } finally {
                result.finish()
            }
        }.start()
    }
}

/** After a reboot the alarm is gone; set it again. */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED && Sunrise.enabled(context)) Sunrise.scheduleNext(context)
    }
}

@Suppress("unused")
private fun epochToExact(ms: Long): ExactInstant {
    val z = ZonedDateTime.ofInstant(Instant.ofEpochMilli(ms), ZoneOffset.UTC)
    return uniffi.kshana_ffi.instantFromUtc(UtcFields(z.year.toLong(), z.monthValue.toUInt(), z.dayOfMonth.toUInt(), z.hour.toUInt(), z.minute.toUInt(), z.second.toUInt(), z.nano.toUInt()))
}
