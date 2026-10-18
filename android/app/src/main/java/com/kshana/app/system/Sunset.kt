package com.kshana.app.system

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import com.kshana.app.data.PredictionStore
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import kotlinx.coroutines.runBlocking
import java.time.Instant

/**
 * The proof feed: at sunrise the day's named hours are remembered as predictions; at
 * sunset the person is asked, in one notification, whether they came true. Grading
 * the app is the ritual that makes the record, and the record is what the person
 * comes back to check.
 */
object Sunset {
    private const val REQUEST = 4102
    private const val K_NEXT = "sunset.next"

    private fun pending(context: Context): PendingIntent {
        val i = Intent(context, SunsetReceiver::class.java).setAction("com.kshana.app.SUNSET")
        return PendingIntent.getBroadcast(context, REQUEST, i, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
    }

    /** Remembers today's best hour for each of the four main lives, and arms the sunset alarm. */
    fun rememberTodayAndSchedule(context: Context) {
        val profile = ProfileStore.load(context) ?: return
        val place = ProfileStore.currentLocation(context)
        val settings = ProfileStore.settings(context)
        try {
            val engine = runBlocking { EngineProvider.get(context) }
            val m = engine.forecastMoment(
                profile.birthInstant(), profile.latitude, profile.longitude,
                place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(),
            )
            val lives = listOf("Love", "Career", "Money", "Health")
            for ((i, l) in m.lives.withIndex()) {
                if (l.life !in lives) continue
                val best = m.hours.filter { !it.caution }.maxByOrNull { it.fit.getOrNull(i) ?: 0.0 } ?: continue
                val fit = best.fit.getOrNull(i) ?: continue
                PredictionStore.remember(
                    context, l.life,
                    Instants.toLocal(best.start).toInstant().toEpochMilli(),
                    Instants.toLocal(best.end).toInstant().toEpochMilli(),
                    (0.6 * l.score + 0.4 * fit),
                )
            }
            val sunset = m.sunset?.let { Instants.toLocal(it).toInstant() } ?: return
            val target = if (sunset.isAfter(Instant.now())) sunset else return
            val am = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
            am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, target.toEpochMilli(), pending(context))
            ProfileStore.prefs(context).edit().putLong(K_NEXT, target.toEpochMilli()).apply()
        } catch (_: Exception) {
        }
    }

    fun ask(context: Context) {
        val pending = PredictionStore.pending(context)
        if (pending.isEmpty()) return
        val first = pending.last()
        Alerts.post(
            context, 4102, "Did it happen? · ${pending.size} to grade",
            "The app read ${first.life.lowercase()} at ${first.score.toInt()} today. Say yes or no on the ${first.life} page, and the app learns how true it is for you.",
        )
    }
}

class SunsetReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val result = goAsync()
        Thread {
            try {
                if (Sunrise.enabled(context)) Sunset.ask(context)
            } finally {
                result.finish()
            }
        }.start()
    }
}
