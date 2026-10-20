package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowRight
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.DailyRecord
import uniffi.kshana_ffi.HourWindowRecord
import java.time.DateTimeException
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

private val CLOCK = DateTimeFormatter.ofPattern("HH:mm")

/** The fourth tab: today's good hours for the active profile, then three doors. */
@Composable
fun RightTimeScreen(
    profile: BirthProfile?,
    here: Place,
    settings: EngineSettings,
    onMuhurta: () -> Unit,
    onCalendar: () -> Unit,
    onMoments: () -> Unit,
    onDay: () -> Unit = {},
) {
    val context = LocalContext.current
    val zone = remember(here) { try { com.kshana.app.data.CityClocks.of(here.zoneId) } catch (e: DateTimeException) { ZoneId.systemDefault() } }
    var daily by remember { mutableStateOf<DailyRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(false) }

    LaunchedEffect(profile, here, settings) {
        daily = null
        error = null
        if (profile == null) return@LaunchedEffect
        loading = true
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            daily = withContext(Dispatchers.Default) {
                engine.daily(birth, profile.latitude, profile.longitude, settings.ayanamsa, Instants.now(), here.latitude, here.longitude)
            }
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        loading = false
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("RIGHT TIME", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text("The right time", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "${here.name} · ${localText(Instants.now(), DATE_ONLY, zone)}",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )

        SectionLabel("Today's good hours")
        val d = daily
        if (profile == null) {
            Text(
                "Good hours are personal: they follow the Moon's distance from your natal Moon. Enter a birth under Me and today's windows appear here.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        } else if (error != null) {
            ErrorLine(error)
        } else if (d == null) {
            Text(if (loading) "reading the day for ${profile.name}…" else "", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        } else {
            if (d.headline.isNotBlank()) Text(d.headline, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            Text(
                "tara ${d.tara} · Moon in house ${d.chandraHouse} · ${d.moonNakshatra}",
                style = MaterialTheme.typography.bodySmall, color = if (d.taraGood && d.chandraGood) KshanaColors.Gold else KshanaColors.Muted,
            )
            if (d.windows.isEmpty()) {
                Text("No marked hours today.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            for (w in d.windows.sortedBy { it.start.seconds() }) WindowLine(w, zone)
        }

        Spacer(Modifier.height(8.dp))
        SectionLabel("Four doors")
        Door("Pick a moment for something", "marriage, business, travel, a new home…", onMuhurta)
        Door("Calendar", "Ekadashi, Pradosha, full and new Moons, Sankranti, with exact times", onCalendar)
        Door("Exact moments", "every ingress, station and lunation to the microsecond", onMoments)
        Door("The day's clocks", "sixty ghatis from the solved sunrise, thirty muhurtas, sixteen choghadiya, your bird's windows", onDay)
    }
}

/** One hour window: start–end in mono, the reason, and "avoid" in gold when it is one to skip. */
@Composable
private fun WindowLine(w: HourWindowRecord, zone: ZoneId) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(
            "${localText(w.start, CLOCK, zone)}–${localText(w.end, CLOCK, zone)}",
            fontFamily = KshanaFonts.mono, fontSize = 12.sp,
            color = if (w.avoid) KshanaColors.Muted else KshanaColors.Sky,
            modifier = Modifier.width(104.dp),
        )
        Text(w.reason, style = MaterialTheme.typography.bodyMedium, color = if (w.avoid) KshanaColors.Muted else KshanaColors.TextSoft, modifier = Modifier.weight(1f))
        if (w.avoid) Text("avoid", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(start = 8.dp))
    }
}

/** A tappable card: serif title, one line of description, an arrow. */
@Composable
private fun Door(title: String, description: String, onClick: () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    Row(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).clickable(onClick = onClick).padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
            Text(description, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = KshanaColors.Gold)
    }
}
