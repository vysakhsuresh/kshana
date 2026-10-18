package com.kshana.app.system

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.widget.RemoteViews
import com.kshana.app.MainActivity
import com.kshana.app.R
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.Instants
import uniffi.kshana_ffi.BriefingRecord
import java.time.Duration
import java.time.Instant

/**
 * The home-screen widget: Saturn's phase with the exact days left (or the day the next
 * phase begins), the moment's name and overall score, the strongest life. It shows the
 * last briefing the app or the sunrise alarm computed; tapping opens the Loop.
 */
class SadeSatiWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, appWidgetManager: AppWidgetManager, appWidgetIds: IntArray) {
        for (id in appWidgetIds) appWidgetManager.updateAppWidget(id, views(context))
    }

    companion object {
        private const val K_LINE1 = "widget.line1"
        private const val K_LINE2 = "widget.line2"
        private const val K_LINE3 = "widget.line3"
        private const val K_LINE4 = "widget.line4"
        private const val K_SCORE = "widget.score"

        /** Stores the lines from a briefing and refreshes every widget instance. */
        fun push(context: Context, b: BriefingRecord) {
            val until = b.saturnUntil?.let { Instants.toLocal(it).toInstant() }
            val days = until?.let { Duration.between(Instant.now(), it).toDays() }
            val line1 = when {
                b.saturnActive && days != null -> "Testing time · $days days left"
                b.saturnActive -> "Testing time"
                else -> "No long test running"
            }
            val m = b.moment
            val top = m.lives.maxByOrNull { it.score }
            val line2 = "${m.title} · ${m.overall.toInt()}"
            val line3 = top?.let { "${it.life}: ${it.headline}" } ?: ""
            val good = m.nextGood.withIndex().filter { it.value != null }.minByOrNull { it.value!!.start.taiSeconds }
            val hm = java.time.format.DateTimeFormatter.ofPattern("HH:mm")
            val line4 = listOfNotNull(
                good?.let { g -> val l = listOf("Love", "Career", "Money", "Health", "Family", "Study", "Home", "Travel", "Children").getOrNull(g.index) ?: ""; "Next good hour: ${Instants.toLocal(g.value!!.start).format(hm)} for ${l.lowercase()}" },
                com.kshana.app.data.RemedyPlans.todayLine(context),
            ).joinToString(" · ")
            ProfileStore.prefs(context).edit()
                .putString(K_LINE1, line1).putString(K_LINE2, line2).putString(K_LINE3, line3).putString(K_LINE4, line4).putInt(K_SCORE, m.overall.toInt())
                .apply()
            val mgr = AppWidgetManager.getInstance(context)
            val ids = mgr.getAppWidgetIds(ComponentName(context, SadeSatiWidget::class.java))
            for (id in ids) mgr.updateAppWidget(id, views(context))
        }

        private fun views(context: Context): RemoteViews {
            val p = ProfileStore.prefs(context)
            val v = RemoteViews(context.packageName, R.layout.widget_sade_sati)
            v.setTextViewText(R.id.widget_line1, p.getString(K_LINE1, "Open Kshana once to fill this"))
            v.setTextViewText(R.id.widget_line2, p.getString(K_LINE2, ""))
            v.setTextViewText(R.id.widget_line3, p.getString(K_LINE3, ""))
            v.setTextViewText(R.id.widget_line4, p.getString(K_LINE4, ""))
            val open = PendingIntent.getActivity(context, 1, Intent(context, MainActivity::class.java), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            v.setOnClickPendingIntent(R.id.widget_root, open)
            return v
        }
    }
}
