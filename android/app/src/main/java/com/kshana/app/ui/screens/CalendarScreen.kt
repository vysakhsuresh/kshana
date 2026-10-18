package com.kshana.app.ui.screens

import android.Manifest
import android.content.ContentUris
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.provider.CalendarContract
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/** An appointment from the phone's calendar, read and never stored. */
internal data class CalEvent(val id: Long, val title: String, val begin: Long, val end: Long)

/** One appointment with how its hour stands for the person. */
internal data class CalCheck(
    val event: CalEvent,
    val score: Double,
    val window: String?,
    val dayWords: String,
    val better: Pair<Long, Long>?,
    val betterScore: Double,
)

internal object CalendarCheck {
    private const val K_ON = "calendar.on"
    fun on(context: Context) = ProfileStore.prefs(context).getBoolean(K_ON, false)
    fun set(context: Context, v: Boolean) = ProfileStore.prefs(context).edit().putBoolean(K_ON, v).apply()
    fun permitted(context: Context) = ContextCompat.checkSelfPermission(context, Manifest.permission.READ_CALENDAR) == PackageManager.PERMISSION_GRANTED

    /** Appointments in the next `days` days, timed ones only. */
    fun events(context: Context, days: Int = 7): List<CalEvent> {
        if (!permitted(context)) return emptyList()
        val now = System.currentTimeMillis()
        val b = CalendarContract.Instances.CONTENT_URI.buildUpon()
        ContentUris.appendId(b, now)
        ContentUris.appendId(b, now + days * 86_400_000L)
        val out = ArrayList<CalEvent>()
        runCatching {
            context.contentResolver.query(
                b.build(),
                arrayOf(CalendarContract.Instances.EVENT_ID, CalendarContract.Instances.TITLE, CalendarContract.Instances.BEGIN, CalendarContract.Instances.END, CalendarContract.Instances.ALL_DAY),
                null, null, "${CalendarContract.Instances.BEGIN} ASC",
            )?.use { c ->
                while (c.moveToNext()) {
                    if (c.getInt(4) == 1) continue
                    out.add(CalEvent(c.getLong(0), c.getString(1) ?: "Appointment", c.getLong(2), c.getLong(3)))
                }
            }
        }
        return out.take(20)
    }
}

/**
 * Your week's appointments, each set against your day: the ones that fall in a weak
 * hour or an unlucky stretch are marked, with the best hour that day to move them to.
 * The calendar is read on the phone and never stored or sent.
 */
@Composable
fun CalendarScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    var allowed by remember { mutableStateOf(CalendarCheck.permitted(context) && CalendarCheck.on(context)) }
    var checks by remember { mutableStateOf<List<CalCheck>?>(null) }
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { ok ->
        CalendarCheck.set(context, ok)
        allowed = ok
    }
    LaunchedEffect(allowed) {
        if (!allowed) return@LaunchedEffect
        val evs = withContext(Dispatchers.IO) { CalendarCheck.events(context) }
        val engine = EngineProvider.get(context)
        val out = ArrayList<CalCheck>()
        for (e in evs) {
            runCatching {
                val at = Instants.fromInstant(Instant.ofEpochMilli(e.begin))
                val m = loadMoment(context, profile, place, settings, at)
                val w = m.windows.firstOrNull { it.caution && e.begin >= it.start.toJava().toEpochMilli() && e.begin < it.end.toJava().toEpochMilli() }
                val dayStart = Instant.ofEpochMilli(e.begin).atZone(ZoneId.systemDefault()).toLocalDate().atStartOfDay(ZoneId.systemDefault()).toInstant()
                val slot = withContext(Dispatchers.Default) {
                    engine.bestSlots(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, "meeting", Instants.fromInstant(dayStart), 1u, 1u)
                }.firstOrNull()
                out.add(
                    CalCheck(
                        e, m.overall, w?.label, m.taraWords,
                        slot?.let { Pair(it.start.toJava().toEpochMilli(), it.end.toJava().toEpochMilli()) }, slot?.score ?: 0.0,
                    ),
                )
            }
        }
        checks = out
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("YOUR CALENDAR", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Your week's appointments", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Each meeting, trip or call in your phone's calendar for the next seven days, set against how that hour stands for you. Weak ones get a better hour the same day. The calendar is read on this phone only; nothing is stored or sent.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        if (!allowed) {
            GoldButton("CHECK MY CALENDAR", onClick = { launcher.launch(Manifest.permission.READ_CALENDAR) }, modifier = Modifier.fillMaxWidth())
            Text("You can turn this off any time in Settings.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            return@Column
        }
        val list = checks
        if (list == null) {
            Working("reading your week…")
            return@Column
        }
        if (list.isEmpty()) Text("No timed appointments in the next seven days.", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.TextSoft)
        val weak = list.count { it.score < 44 || it.window != null }
        if (list.isNotEmpty()) Text(
            if (weak == 0) "All ${list.size} sit in reasonable hours." else "$weak of ${list.size} sit in a weak hour. A better hour is suggested for each.",
            fontFamily = FontFamily.Serif, fontSize = 19.sp, lineHeight = 26.sp, color = KshanaColors.Text,
        )
        val hm = DateTimeFormatter.ofPattern("HH:mm")
        val day = DateTimeFormatter.ofPattern("EEE d MMM")
        val z = ZoneId.systemDefault()
        for (c in list) {
            val bad = c.score < 44 || c.window != null
            Column(Modifier.fillMaxWidth().glass(14.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(c.event.title, style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                    Text(c.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = scoreColor(c.score))
                }
                Text("${Instant.ofEpochMilli(c.event.begin).atZone(z).format(day)} · ${Instant.ofEpochMilli(c.event.begin).atZone(z).format(hm)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
                Text(
                    when {
                        c.window != null -> "It falls in a ${c.window.lowercase()}. Things begun now tend to stall."
                        c.score < 44 -> "A weak hour for you: expect friction or delay."
                        c.score >= 56 -> "A good hour for you. Keep it."
                        else -> "An even hour. Fine to keep."
                    },
                    style = MaterialTheme.typography.bodyMedium, color = if (bad) KshanaColors.Ember else KshanaColors.TextSoft,
                )
                Text(c.dayWords.replaceFirstChar { it.uppercase() }.trimEnd('.') + ".", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                val b = c.better
                if (bad && b != null && c.betterScore > c.score) {
                    Text("Better that day: ${Instant.ofEpochMilli(b.first).atZone(z).format(hm)} – ${Instant.ofEpochMilli(b.second).atZone(z).format(hm)} (${c.betterScore.toInt()})", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Leaf)
                }
                Text(
                    "OPEN IN CALENDAR ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                    modifier = Modifier.clickable {
                        val uri = ContentUris.withAppendedId(CalendarContract.Events.CONTENT_URI, c.event.id)
                        runCatching { context.startActivity(Intent(Intent.ACTION_VIEW).setData(uri)) }
                    }.padding(vertical = 4.dp),
                )
            }
        }
    }
}
