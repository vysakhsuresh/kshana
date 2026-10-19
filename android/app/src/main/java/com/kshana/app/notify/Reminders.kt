package com.kshana.app.notify

import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import com.kshana.app.data.FamilyStore
import org.json.JSONArray
import org.json.JSONObject
import com.kshana.app.ui.sky.Haptics

/**
 * Exact-moment reminders: one alarm per moment, a notification when it fires. Where the
 * user has allowed exact alarms the reminder is set as an alarm clock (exact, allowed while
 * idle, shown by the system as a pending alarm); otherwise Android places it in an inexact
 * slot and Settings says so. The scheduled set is persisted so a reboot can put them back.
 * No library beyond the framework.
 */
object Reminders {
    const val CHANNEL_ID = "kshana.moments"
    const val CHANNEL_NAME = "Exact moments"
    const val EXTRA_ID = "kshana.reminder.id"
    const val EXTRA_TITLE = "kshana.reminder.title"
    const val EXTRA_TEXT = "kshana.reminder.text"

    /** One persisted reminder: a stable id, when it fires (epoch millis) and what it says. */
    data class Entry(val id: Int, val atMillis: Long, val title: String, val text: String)

    /** A non-negative id derived from a key, the same on every run. */
    fun stableId(key: String): Int = key.hashCode() and 0x7FFFFFFF

    /** Schedule (or move) the reminder `id` to `atMillis`, and remember it. */
    fun schedule(context: Context, id: Int, atMillis: Long, title: String, text: String) {
        ensureChannel(context)
        val pi = pending(context, id, title, text)
        val am = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        if (exactAllowed(context)) {
            val launch = context.packageManager.getLaunchIntentForPackage(context.packageName)
            val show = if (launch != null) {
                PendingIntent.getActivity(context, id, launch, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
            } else pi
            am.setAlarmClock(AlarmManager.AlarmClockInfo(atMillis, show), pi)
        } else {
            am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMillis, pi)
        }
        val kept = load(context).filter { it.id != id } + Entry(id, atMillis, title, text)
        save(context, kept)
    }

    /** Cancel the alarm for `id` and forget it. */
    fun cancel(context: Context, id: Int) {
        val pi = pending(context, id, "", "")
        val am = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        am.cancel(pi)
        pi.cancel()
        save(context, load(context).filter { it.id != id })
    }

    /** Whether the system lets this app set exact alarms (always, before Android 12). */
    fun exactAllowed(context: Context): Boolean {
        if (Build.VERSION.SDK_INT < 31) return true
        val am = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        return am.canScheduleExactAlarms()
    }

    /** Open the system page where the user can allow exact alarms for this app. */
    fun requestExact(context: Context) {
        if (Build.VERSION.SDK_INT < 31) return
        runCatching {
            val intent = Intent(android.provider.Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM)
                .setData(android.net.Uri.parse("package:" + context.packageName))
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(intent)
        }
    }

    /** The haptic phrase for a reminder, from the words it carries. */
    fun hapticKind(title: String, text: String): String {
        val w = (title + " " + text).lowercase()
        return when {
            "dasha" in w || "antar" in w || "pratyantar" in w -> "dasha"
            "sunrise" in w -> "sunrise"
            "sunset" in w -> "sunset"
            "hora" in w -> "hora"
            "rahu" in w -> "rahu"
            "muhurta" in w || "choghadiya" in w -> "muhurta"
            else -> "moon"
        }
    }

    /** True while a future reminder with this id is on the books. */
    fun isScheduled(context: Context, id: Int): Boolean = load(context).any { it.id == id }

    /** Every persisted reminder still in the future, soonest first. */
    fun scheduled(context: Context): List<Entry> = load(context).sortedBy { it.atMillis }

    /** Create the channel once; re-creating with the same id is a no-op on Android. */
    fun ensureChannel(context: Context) {
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (nm.getNotificationChannel(CHANNEL_ID) == null) {
            val channel = NotificationChannel(CHANNEL_ID, CHANNEL_NAME, NotificationManager.IMPORTANCE_HIGH)
            channel.description = "A moment in the sky, to the second"
            nm.createNotificationChannel(channel)
        }
    }

    /** Whether a posted notification would reach the user. */
    fun canPostNotifications(context: Context): Boolean {
        if (Build.VERSION.SDK_INT >= 33 &&
            context.checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) return false
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        return nm.areNotificationsEnabled()
    }

    /** Re-arm every persisted future reminder (after a reboot or an app update). */
    fun rescheduleAll(context: Context) {
        for (e in load(context)) schedule(context, e.id, e.atMillis, e.title, e.text)
    }

    /** Forget a reminder that has fired. */
    internal fun forget(context: Context, id: Int) = save(context, load(context).filter { it.id != id })

    private fun pending(context: Context, id: Int, title: String, text: String): PendingIntent {
        val intent = Intent(context, ReminderReceiver::class.java)
            .setAction("com.kshana.app.REMIND")
            .setData(android.net.Uri.parse("kshana://reminder/$id"))
            .putExtra(EXTRA_ID, id)
            .putExtra(EXTRA_TITLE, title)
            .putExtra(EXTRA_TEXT, text)
        return PendingIntent.getBroadcast(context, id, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
    }

    /** The persisted set with past entries pruned. */
    private fun load(context: Context): List<Entry> {
        val now = System.currentTimeMillis()
        val raw = FamilyStore.remindersJson(context)
        val list = runCatching {
            val arr = JSONArray(raw)
            (0 until arr.length()).map { i ->
                val o = arr.getJSONObject(i)
                Entry(o.optInt("id"), o.optLong("atMillis"), o.optString("title", ""), o.optString("text", ""))
            }
        }.getOrDefault(emptyList())
        val future = list.filter { it.atMillis > now }
        if (future.size != list.size) save(context, future)
        return future
    }

    private fun save(context: Context, list: List<Entry>) {
        val arr = JSONArray()
        for (e in list) {
            arr.put(JSONObject().put("id", e.id).put("atMillis", e.atMillis).put("title", e.title).put("text", e.text))
        }
        FamilyStore.saveRemindersJson(context, arr.toString())
    }
}

/** Fires at the scheduled instant: posts the notification and forgets the entry. */
class ReminderReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val id = intent.getIntExtra(Reminders.EXTRA_ID, 0)
        val title = intent.getStringExtra(Reminders.EXTRA_TITLE) ?: "Kshana"
        val text = intent.getStringExtra(Reminders.EXTRA_TEXT) ?: ""
        Reminders.ensureChannel(context)
        val launch = context.packageManager.getLaunchIntentForPackage(context.packageName)
            ?.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        val content = if (launch != null) {
            PendingIntent.getActivity(context, id, launch, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        } else null
        val builder = Notification.Builder(context, Reminders.CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(title)
            .setContentText(text)
            .setAutoCancel(true)
            .setShowWhen(true)
        if (content != null) builder.setContentIntent(content)
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Reminders.canPostNotifications(context)) nm.notify(id, builder.build())
        runCatching { Haptics.exactInstant(context, Reminders.hapticKind(title, text), alarm = true) }
        Reminders.forget(context, id)
    }
}

/** After a reboot every alarm is gone; put the persisted future ones back. */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED || intent.action == Intent.ACTION_MY_PACKAGE_REPLACED) {
            Reminders.rescheduleAll(context)
        }
    }
}
