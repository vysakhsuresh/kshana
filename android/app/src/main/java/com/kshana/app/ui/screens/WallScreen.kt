package com.kshana.app.ui.screens

import android.content.Intent
import android.provider.CalendarContract
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.system.Alerts
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.CountdownRecord
import uniffi.kshana_ffi.FestivalRecord
import java.time.Duration
import java.time.Instant

internal object WallCache {
    var key: String? = null
    var countdowns: List<CountdownRecord>? = null
    var festivals: List<FestivalRecord>? = null
}

/** "d h m" or "h m s" for the time left. */
internal fun left(now: Instant, to: Instant): String {
    val s = Duration.between(now, to).seconds
    if (s <= 0) return "now"
    val d = s / 86_400
    val h = (s % 86_400) / 3600
    val m = (s % 3600) / 60
    val sec = s % 60
    return when {
        d >= 100 -> "$d days"
        d > 0 -> "${d}d ${h}h"
        h > 0 -> "${h}h ${m}m ${sec}s"
        else -> "${m}m ${sec}s"
    }
}

/** Opens the phone's calendar with the event filled in. */
internal fun addToCalendar(context: android.content.Context, title: String, startMillis: Long, endMillis: Long?, notes: String) {
    val i = Intent(Intent.ACTION_INSERT).setData(CalendarContract.Events.CONTENT_URI)
        .putExtra(CalendarContract.Events.TITLE, title)
        .putExtra(CalendarContract.EXTRA_EVENT_BEGIN_TIME, startMillis)
        .putExtra(CalendarContract.EXTRA_EVENT_END_TIME, endMillis ?: (startMillis + 3_600_000L))
        .putExtra(CalendarContract.Events.DESCRIPTION, notes)
    runCatching { context.startActivity(i) }
}

/**
 * The wall of countdowns: every exact instant the person is waiting for, as a big
 * number that ticks, with a bell to be told and a way into the calendar. Under it,
 * the month's observances.
 */
@Composable
fun WallScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit, onRemedy: () -> Unit = {}) {
    val context = LocalContext.current
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|${settings.ayanamsa}|${java.time.LocalDate.now()}"
    var items by remember { mutableStateOf(if (WallCache.key == key) WallCache.countdowns else null) }
    var fest by remember { mutableStateOf(if (WallCache.key == key) WallCache.festivals else null) }
    var error by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableStateOf(Instant.now()) }
    var version by remember { mutableStateOf(0) }
    LaunchedEffect(Unit) { while (true) { tick = Instant.now(); delay(1000) } }
    LaunchedEffect(key) {
        if (items != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val (c, f) = withContext(Dispatchers.Default) {
                Pair(
                    engine.countdowns(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now()),
                    engine.festivalsHere(profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa, place.latitude, place.longitude, com.kshana.app.data.PlaceClock.offsetMinutes(place), Instants.now(), 35u),
                )
            }
            WallCache.key = key
            WallCache.countdowns = c
            WallCache.festivals = f
            items = c
            fest = f
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("THE WALL", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("What you are waiting for", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("The days that will change things for you, counting down. Tap the bell to be told when one comes; the calendar mark puts it beside your other days.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        ErrorLine(error)
        val list = items
        if (list == null) {
            if (error == null) Working("looking ahead…")
            return@Column
        }
        for (c in list) {
            val at = Instants.toLocal(c.at).toInstant()
            val set = remember(version, c.at.taiSeconds) { Alerts.has(context, at.toEpochMilli()) }
            Column(
                Modifier.fillMaxWidth().glass(16.dp)
                    .then(if (c.kind == "saturn" || c.kind == "sadesati") Modifier.border(1.dp, (if (c.good) KshanaColors.Leaf else KshanaColors.Ember).copy(alpha = 0.6f), RoundedCornerShape(16.dp)) else Modifier)
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(c.title.uppercase(), style = MaterialTheme.typography.labelSmall, color = if (c.good) KshanaColors.Leaf else KshanaColors.Ember)
                Text(left(tick, at), fontFamily = FontFamily.Monospace, fontSize = 34.sp, color = KshanaColors.Text)
                Text("${localText(c.at, DATE_TIME)} · ${localText(c.at, java.time.format.DateTimeFormatter.ofPattern("EEEE"))}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
                Text(c.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Chip(if (set) "🔔 TELLING YOU" else "🔔 TELL ME", selected = set, onClick = {
                        Alerts.toggle(context, at.toEpochMilli(), c.title, c.words)
                        version++
                    })
                    Chip("CALENDAR", selected = false, onClick = { addToCalendar(context, "Kshana · ${c.title}", at.toEpochMilli(), null, c.words) })
                    // A day that weighs is never left on the wall alone: what helps is one tap away.
                    if (!c.good) Chip("WHAT HELPS", selected = false, onClick = onRemedy)
                }
            }
        }
        val f = fest
        if (f != null && f.isNotEmpty()) {
            SectionLabel("Holy days this month")
            for (x in f) {
                val at = Instants.toLocal(x.at).toInstant()
                Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.Top) {
                    Column(Modifier.width(96.dp)) {
                        Text(localText(x.at, WEEKDAY_DAY), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
                        Text(localText(x.at, TIME_HM), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Muted)
                    }
                    Column(Modifier.weight(1f)) {
                        Text(x.name, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
                        Text(x.what, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            Text("TELL ME", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { Alerts.toggle(context, at.toEpochMilli() - 3_600_000L, x.name, x.what); version++ }.padding(vertical = 4.dp))
                            Text("CALENDAR", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { addToCalendar(context, "Kshana · ${x.name}", at.toEpochMilli(), x.end?.let { Instants.toLocal(it).toInstant().toEpochMilli() }, x.what) }.padding(vertical = 4.dp))
                        }
                    }
                }
            }
        }
    }
}
