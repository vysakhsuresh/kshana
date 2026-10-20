package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.lamp.TodayLamp
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.DailyRecord
import uniffi.kshana_ffi.EventRecord
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.HourWindowRecord
import uniffi.kshana_ffi.TransitRecord
import uniffi.kshana_ffi.TransitReport
import java.time.DateTimeException
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

private val GREETING_DATE = DateTimeFormatter.ofPattern("EEEE, d MMMM uuuu")
private val SHORT_TIME = DateTimeFormatter.ofPattern("HH:mm")
private val MOMENT_TIME = DateTimeFormatter.ofPattern("EEE d MMM · HH:mm:ss")

/** A dim red for the hours to avoid; the only non-palette colour, kept to the day bar. */
private val AvoidFill = Color(0xFF7A3238)

/**
 * The front door: today for this person, where they are now. The day bar runs from
 * sunrise to the next sunrise; the needle moves every 30 seconds.
 */
@Composable
fun TodayScreen(
    profile: BirthProfile,
    here: Place,
    settings: EngineSettings,
    simple: Boolean,
    onAsk: () -> Unit,
    onRightTime: () -> Unit,
    onLife: () -> Unit,
    onRiver: () -> Unit,
    onSettings: () -> Unit,
) {
    val context = LocalContext.current
    val zone = remember(here, profile) {
        try { ZoneId.of(here.zoneId) } catch (e: DateTimeException) { profile.zone() }
    }
    var now by remember { mutableStateOf(Instants.now()) }
    var daily by remember { mutableStateOf<DailyRecord?>(null) }
    var transits by remember { mutableStateOf<TransitReport?>(null) }
    var events by remember { mutableStateOf<List<EventRecord>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(true) }

    // The needle: "now" moves every 30 s without recomputing anything.
    LaunchedEffect(Unit) {
        while (true) {
            delay(30_000)
            now = Instants.now()
        }
    }

    // Recompute when the person, the place, the settings or the local date change.
    val dayKey = remember(now, zone) { Instants.toLocal(now, zone).toLocalDate() }
    LaunchedEffect(profile, here, settings, dayKey) {
        loading = true
        val problems = mutableListOf<String>()
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val at = Instants.now()
            now = at
            try {
                daily = withContext(Dispatchers.Default) {
                    engine.daily(birth, profile.latitude, profile.longitude, settings.ayanamsa, at, here.latitude, here.longitude)
                }
            } catch (e: Exception) {
                problems += e.message ?: e.toString()
            }
            try {
                transits = withContext(Dispatchers.Default) {
                    engine.transits(birth, profile.latitude, profile.longitude, settings.ayanamsa, at, 30u)
                }
            } catch (e: Exception) {
                problems += e.message ?: e.toString()
            }
            try {
                events = withContext(Dispatchers.Default) { engine.nextEvents(at, 7u, settings.ayanamsa) }
                    .sortedBy { it.instant.seconds() }
                    .take(3)
            } catch (e: Exception) {
                problems += e.message ?: e.toString()
            }
        } catch (e: Exception) {
            problems += e.message ?: e.toString()
        }
        error = problems.distinct().joinToString(" · ").ifEmpty { null }
        loading = false
    }

    val local = remember(now, zone) { Instants.toLocal(now, zone) }
    val firstName = remember(profile) { profile.name.trim().split(Regex("\\s+")).firstOrNull().orEmpty() }
    val greeting = remember(local.hour, firstName) {
        val word = when (local.hour) { in 5..11 -> "Good morning"; in 12..16 -> "Good afternoon"; in 17..21 -> "Good evening"; else -> "Good night" }
        if (firstName.isEmpty()) word else "$word, $firstName"
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("TODAY · ${here.name.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(greeting, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
                Text(local.format(GREETING_DATE), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            Box(Modifier.size(44.dp).clickable(onClick = onSettings), contentAlignment = Alignment.Center) {
                Icon(Icons.Filled.Settings, contentDescription = "Settings", tint = KshanaColors.Muted)
            }
        }
        ErrorLine(error)

        val d = daily
        if (d == null) {
            if (loading) Text("Reading the sky for today…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            Text(d.headline, fontFamily = KshanaFonts.serif, fontSize = 22.sp, lineHeight = 30.sp, color = KshanaColors.Text, modifier = Modifier.padding(top = 6.dp))

            TodayLamp(profile, here, settings)
            SectionLabel("Your day")
            val sunrise = d.sunrise
            val nextSunrise = d.nextSunrise
            if (sunrise != null && nextSunrise != null) {
                DayBar(sunrise, d.sunset, nextSunrise, d.windows, now)
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text("rise ${localText(sunrise, SHORT_TIME, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                    d.sunset?.let { Text("set ${localText(it, SHORT_TIME, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted) }
                    Text("rise ${localText(nextSunrise, SHORT_TIME, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                }
            } else {
                Text("The Sun does not rise and set here today, so there are no hour windows.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            for (w in d.windows) WindowRow(w, now, zone)

            SectionLabel("The sky and you")
            SkyCard(transits, zone, loading)

            if (events.isNotEmpty()) {
                SectionLabel("Next exact moments")
                for (e in events) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text(momentTitle(e), fontFamily = KshanaFonts.serif, fontSize = 16.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                        Text(localText(e.instant, MOMENT_TIME, zone), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
                    }
                }
            }

            SectionLabel("The river, next")
            NextChanges(profile, here, settings, count = 5)
            Row(
                Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable(onClick = onRiver),
                horizontalArrangement = Arrangement.Start, verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("OPEN THE RIVER · EVERY EXACT INSTANT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = KshanaColors.Gold, modifier = Modifier.size(18.dp))
            }

            if (!simple) {
                SectionLabel("Exact")
                MonoRow("Sunrise", sunrise?.let { localText(it, TIME_ONLY, zone) } ?: "—")
                MonoRow("Sunset", d.sunset?.let { localText(it, TIME_ONLY, zone) } ?: "—")
                MonoRow("Next sunrise", nextSunrise?.let { localText(it, TIME_ONLY, zone) } ?: "—")
                MonoRow("Moon nakshatra", d.moonNakshatra)
                MonoRow("Tara", "${d.tara} · ${if (d.taraGood) "good" else "unfavourable"}", if (d.taraGood) KshanaColors.Gold else KshanaColors.Sky)
                MonoRow("Chandra bala", "${ordinal(d.chandraHouse.toInt())} from Moon · ${if (d.chandraGood) "good" else "weak"}", if (d.chandraGood) KshanaColors.Gold else KshanaColors.Sky)
                Text("${zone.id} · judged at ${localText(now, DATE_TIME_S, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            }
        }

        Spacer(Modifier.height(10.dp))
        GoldButton("ASK A QUESTION", onClick = onAsk, modifier = Modifier.fillMaxWidth())
        QuietButton("FIND THE RIGHT TIME", onClick = onRightTime, modifier = Modifier.fillMaxWidth())
        Row(
            Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable(onClick = onLife),
            horizontalArrangement = Arrangement.Center, verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("READ YOUR LIFE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = KshanaColors.Gold, modifier = Modifier.size(18.dp))
        }
    }
}

/** Sunrise to next sunrise: good hours in sky blue, hours to avoid in dim red with a gold edge, a gold needle at now. */
@Composable
private fun DayBar(sunrise: ExactInstant, sunset: ExactInstant?, nextSunrise: ExactInstant, windows: List<HourWindowRecord>, now: ExactInstant) {
    Canvas(Modifier.fillMaxWidth().height(40.dp)) {
        val w = size.width
        val top = 8.dp.toPx()
        val h = size.height - top - 4.dp.toPx()
        fun x(e: ExactInstant): Float = progress(sunrise, nextSunrise, e) * w
        drawRect(KshanaColors.LineSoft, topLeft = Offset(0f, top), size = Size(w, h))
        if (sunset != null) {
            // the night, a shade darker
            val xs = x(sunset)
            drawRect(KshanaColors.Night, topLeft = Offset(xs, top), size = Size(w - xs, h))
            drawLine(KshanaColors.Muted.copy(alpha = 0.6f), Offset(xs, top), Offset(xs, top + h), 1f)
        }
        for (win in windows.filter { !it.avoid }) {
            val a = x(win.start)
            val b = x(win.end)
            drawRect(KshanaColors.Sky.copy(alpha = 0.55f), topLeft = Offset(a, top), size = Size((b - a).coerceAtLeast(1f), h))
        }
        for (win in windows.filter { it.avoid }) {
            val a = x(win.start)
            val b = x(win.end)
            drawRect(AvoidFill.copy(alpha = 0.7f), topLeft = Offset(a, top), size = Size((b - a).coerceAtLeast(1f), h))
            drawRect(KshanaColors.Gold, topLeft = Offset(a, top), size = Size((b - a).coerceAtLeast(1f), h), style = Stroke(1.dp.toPx()))
        }
        drawRect(KshanaColors.Line, topLeft = Offset(0f, top), size = Size(w, h), style = Stroke(1f))
        // the needle
        val xn = x(now)
        drawLine(KshanaColors.Gold, Offset(xn, 0f), Offset(xn, size.height), 2f)
        val tri = Path().apply {
            moveTo(xn - 5.dp.toPx(), 0f)
            lineTo(xn + 5.dp.toPx(), 0f)
            lineTo(xn, top - 1f)
            close()
        }
        drawPath(tri, KshanaColors.Gold)
    }
}

@Composable
private fun WindowRow(w: HourWindowRecord, now: ExactInstant, zone: ZoneId) {
    val nowS = now.seconds()
    val live = nowS >= w.start.seconds() && nowS < w.end.seconds()
    val colour = when {
        w.avoid -> KshanaColors.Gold
        live -> KshanaColors.Text
        else -> KshanaColors.TextSoft
    }
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(
            "${localText(w.start, SHORT_TIME, zone)} – ${localText(w.end, SHORT_TIME, zone)}",
            fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = if (live) KshanaColors.Gold else KshanaColors.Sky,
        )
        Text(
            if (w.avoid) "avoid · ${w.reason}" else w.reason,
            style = MaterialTheme.typography.bodyMedium, color = colour, modifier = Modifier.weight(1f).padding(start = 14.dp),
        )
        if (live) Text("NOW", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
    }
}

/** Sade Sati in one line, then Jupiter and Saturn counted from the natal Moon. */
@Composable
private fun SkyCard(report: TransitReport?, zone: ZoneId, loading: Boolean) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        if (report == null) {
            Text(if (loading) "Placing Saturn and Jupiter against your Moon…" else "The transits could not be read.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            return@Column
        }
        val ss = report.sadeSati
        val phase = ss.phase.toInt()
        if (phase > 0) {
            val until = ss.phaseEnd?.let { localText(it, DATE_ONLY, zone) }
            Text(
                "You are in Sade Sati (phase $phase of 3)${until?.let { " until $it" } ?: ""}.",
                fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Gold,
            )
        } else {
            val next = ss.nextStart?.let { localText(it, DATE_ONLY, zone) }
            Text(
                if (next != null) "Not in Sade Sati; next begins $next." else "Not in Sade Sati, and none begins in the next thirty years.",
                fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text,
            )
        }
        for (body in listOf("Jupiter", "Saturn")) {
            val house = fromMoonHouse(report.transits, body)
            if (house != null) {
                Text("$body in your ${ordinal(house)} from the Moon", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            }
        }
    }
}

/** The house from the natal Moon: the from_moon record, or house 1 when the body sits on the Moon. */
private fun fromMoonHouse(transits: List<TransitRecord>, body: String): Int? {
    transits.firstOrNull { it.body == body && it.kind == "from_moon" }?.let { return it.house.toInt() }
    transits.firstOrNull { it.body == body && it.natal == "Moon" && it.kind == "over" }?.let { return 1 }
    return null
}

private fun ordinal(n: Int): String {
    val suffix = when {
        n % 100 in 11..13 -> "th"
        n % 10 == 1 -> "st"
        n % 10 == 2 -> "nd"
        n % 10 == 3 -> "rd"
        else -> "th"
    }
    return "$n$suffix"
}

private fun momentTitle(e: EventRecord): String = when (e.kind) {
    "ingress" -> "${e.body} changes sign"
    "station" -> "${e.body} stations"
    "new_moon" -> "New Moon · Amavasya"
    "full_moon" -> "Full Moon · Purnima"
    "tithi" -> "Tithi changes"
    "nakshatra" -> "Moon changes nakshatra"
    else -> e.kind
}
