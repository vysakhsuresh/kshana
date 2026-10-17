package com.kshana.app.system

import android.app.AlarmManager
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
import org.json.JSONArray
import org.json.JSONObject

/**
 * Alerts the person chooses: a shift, a window, a good hour, a day on the wall. Each is
 * an exact alarm that posts one notification with the words the engine gave it. The
 * list lives in the phone's preferences and is re-armed on launch and after a reboot.
 */
data class Alert(val id: Int, val at: Long, val title: String, val text: String)

object Alerts {
    private const val KEY = "alerts.json"
    private const val CHANNEL = "alerts"

    fun all(context: Context): List<Alert> {
        val raw = ProfileStore.prefs(context).getString(KEY, null) ?: return emptyList()
        return runCatching {
            val arr = JSONArray(raw)
            (0 until arr.length()).map { i ->
                val o = arr.getJSONObject(i)
                Alert(o.getInt("id"), o.getLong("at"), o.getString("title"), o.getString("text"))
            }
        }.getOrDefault(emptyList())
    }

    private fun save(context: Context, list: List<Alert>) {
        val arr = JSONArray()
        for (a in list) arr.put(JSONObject().put("id", a.id).put("at", a.at).put("title", a.title).put("text", a.text))
        ProfileStore.prefs(context).edit().putString(KEY, arr.toString()).apply()
    }

    fun has(context: Context, at: Long): Boolean = all(context).any { kotlin.math.abs(it.at - at) < 1000L }

    /** Adds (or removes, when one already stands at that instant) an alert. Returns true when set. */
    fun toggle(context: Context, at: Long, title: String, text: String): Boolean {
        val list = all(context).toMutableList()
        val existing = list.firstOrNull { kotlin.math.abs(it.at - at) < 1000L }
        return if (existing != null) {
            cancel(context, existing)
            list.remove(existing)
            save(context, list)
            false
        } else {
            val id = (at / 1000L % 1_000_000L).toInt() + 10_000
            val a = Alert(id, at, title, text)
            list.add(a)
            save(context, list.filter { it.at > System.currentTimeMillis() - 86_400_000L })
            arm(context, a)
            true
        }
    }

    private fun pending(context: Context, a: Alert): PendingIntent {
        val i = Intent(context, AlertReceiver::class.java).setAction("com.kshana.app.ALERT")
            .putExtra("id", a.id).putExtra("title", a.title).putExtra("text", a.text)
        return PendingIntent.getBroadcast(context, a.id, i, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
    }

    private fun arm(context: Context, a: Alert) {
        if (a.at <= System.currentTimeMillis()) return
        val am = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        try {
            if (Build.VERSION.SDK_INT < 31 || am.canScheduleExactAlarms()) {
                am.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, a.at, pending(context, a))
            } else {
                am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, a.at, pending(context, a))
            }
        } catch (_: SecurityException) {
            am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, a.at, pending(context, a))
        }
    }

    private fun cancel(context: Context, a: Alert) {
        (context.getSystemService(Context.ALARM_SERVICE) as AlarmManager).cancel(pending(context, a))
    }

    /** Re-arms every future alert (on launch and after a reboot). */
    fun rearm(context: Context) {
        val now = System.currentTimeMillis()
        val live = all(context).filter { it.at > now }
        save(context, live)
        for (a in live) arm(context, a)
    }

    fun channel(context: Context) {
        if (Build.VERSION.SDK_INT >= 26) {
            val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            nm.createNotificationChannel(android.app.NotificationChannel(CHANNEL, "Alerts you chose", NotificationManager.IMPORTANCE_HIGH).apply {
                description = "A shift, a window or a day you asked to be told about."
            })
        }
    }

    fun post(context: Context, id: Int, title: String, text: String) {
        channel(context)
        val open = PendingIntent.getActivity(context, id, Intent(context, MainActivity::class.java), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        val n = NotificationCompat.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_stat_kshana)
            .setContentTitle(title)
            .setContentText(text)
            .setStyle(NotificationCompat.BigTextStyle().bigText(text))
            .setContentIntent(open)
            .setAutoCancel(true)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .build()
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT < 33 || context.checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) == android.content.pm.PackageManager.PERMISSION_GRANTED) {
            nm.notify(id, n)
        }
    }
}

class AlertReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val id = intent.getIntExtra("id", 0)
        val title = intent.getStringExtra("title") ?: "Kshana"
        val text = intent.getStringExtra("text") ?: ""
        Alerts.post(context, id, title, text)
        // Drop it from the list once fired.
        val remaining = Alerts.all(context).filter { it.id != id }
        ProfileStore.prefs(context).edit().putString("alerts.json", org.json.JSONArray().also { arr ->
            for (a in remaining) arr.put(JSONObject().put("id", a.id).put("at", a.at).put("title", a.title).put("text", a.text))
        }.toString()).apply()
    }
}
