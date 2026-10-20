package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.MuhurtaRecord
import java.time.DateTimeException
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

/** The seven activities the engine knows, by their engine names. */
private val ACTIVITIES = listOf("Marriage", "Travel", "Business", "Housewarming", "Learning", "Medical", "General")

private val RANGES = listOf(7u, 14u, 30u)

private const val LIMIT = 12u

private val DAY = DateTimeFormatter.ofPattern("EEE d MMM")
private val CLOCK = DateTimeFormatter.ofPattern("HH:mm")

/** The natal Moon as the engine states it: sidereal longitude and the nakshatra it sits in. */
private data class NatalMoon(val sidereal: Double, val nakshatra: String)

/** Two taps and a scan: activity, range; then the windows the rules score highest, best first. */
@Composable
fun MuhurtaScreen(place: Place, settings: EngineSettings, profile: BirthProfile?, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val zone = remember(place) { try { ZoneId.of(place.zoneId) } catch (e: DateTimeException) { ZoneId.systemDefault() } }
    var activity by remember { mutableStateOf<String?>(null) }
    var days by remember { mutableStateOf(7u) }
    var natal by remember { mutableStateOf<NatalMoon?>(null) }
    var scanning by remember { mutableStateOf(false) }
    var scanned by remember { mutableStateOf(false) }
    var windows by remember { mutableStateOf<List<MuhurtaRecord>>(emptyList()) }
    var status by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    // The natal Moon is read once from the birth chart; nothing else of the chart is used here.
    LaunchedEffect(profile, settings) {
        natal = null
        if (profile == null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val chart = withContext(Dispatchers.Default) {
                engine.chart(profile.birthInstant(), profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
            }
            natal = NatalMoon(chart.panchanga.moonSidereal, chart.panchanga.nakshatra)
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    val ready = activity != null && !scanning

    fun find() {
        val a = activity ?: return
        val n = days
        scanning = true
        scanned = false
        error = null
        status = "scanning $n days in five-minute steps…"
        scope.launch {
            try {
                val engine = EngineProvider.get(context)
                val found = withContext(Dispatchers.Default) {
                    engine.muhurta(a, Instants.now(), n, place.latitude, place.longitude, settings.ayanamsa, natal?.sidereal, LIMIT)
                }
                windows = found
                scanned = true
                status = if (found.isEmpty()) null else "${found.size} windows · best first · $n days from now"
            } catch (e: Exception) {
                error = e.message ?: e.toString()
                status = null
            }
            scanning = false
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("MUHURTA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Find the right time", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("Classical rules, your natal Moon, the sky at your place. Nothing else.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        Text("${place.name} · ${"%.4f".format(place.latitude)}° ${"%.4f".format(place.longitude)}°", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)

        SectionLabel("Activity")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (name in ACTIVITIES) Chip(name.uppercase(), selected = activity == name, onClick = { activity = name })
        }

        SectionLabel("Range")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (n in RANGES) Chip("$n DAYS", selected = days == n, onClick = { days = n })
        }

        val moon = natal
        Text(
            if (profile == null) "Add your birth details in Settings to include Tara bala and Chandra bala"
            else if (moon == null) "Reading your natal Moon…"
            else "Tara bala and Chandra bala from your natal Moon in ${moon.nakshatra}",
            style = MaterialTheme.typography.bodySmall, color = if (moon != null) KshanaColors.Sky else KshanaColors.Muted,
        )

        Spacer(Modifier.height(8.dp))
        ErrorLine(error)
        GoldButton(if (scanning) "SCANNING…" else "FIND WINDOWS", enabled = ready, onClick = { find() }, modifier = Modifier.fillMaxWidth())
        status?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted) }

        if (scanned && windows.isEmpty()) {
            Text(
                "No window scored above the threshold in this range. Widen the range or choose General.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp),
            )
        }
        for (w in windows) WindowCard(w, zone)

        Spacer(Modifier.height(12.dp))
        Text(
            "A muhurta is a recommendation from the tradition's rules, listed here in full. The sky is exact; the choice is yours.",
            fontFamily = KshanaFonts.serif, fontStyle = FontStyle.Italic, fontSize = 14.sp, lineHeight = 20.sp, color = KshanaColors.Muted,
        )
    }
}

/** One window: when, how strong, what the sky holds, and every rule that spoke for or against it. */
@Composable
private fun WindowCard(w: MuhurtaRecord, zone: ZoneId) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(localText(w.start, DAY, zone), fontFamily = KshanaFonts.serif, fontSize = 18.sp, color = KshanaColors.Text)
            if (w.abhijit) {
                Text(
                    "ABHIJIT",
                    style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                    modifier = Modifier.border(1.dp, KshanaColors.Gold, RoundedCornerShape(22.dp)).padding(horizontal = 10.dp, vertical = 6.dp),
                )
            }
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("${localText(w.start, CLOCK, zone)} → ${localText(w.end, CLOCK, zone)}", fontFamily = KshanaFonts.mono, fontSize = 13.sp, color = KshanaColors.Sky)
            RemindButton(
                key = "muhurta:${w.start.taiSeconds}",
                atMillis = Instants.toLocal(w.start, zone).toInstant().toEpochMilli() - 10L * 60L * 1000L,
                title = "Your window opens at ${localText(w.start, TIME_ONLY, zone)}",
                text = "${localText(w.start, DATE_TIME_S, zone)} → ${localText(w.end, TIME_ONLY, zone)} · ${w.tithi} · ${w.nakshatra}",
            )
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            ScoreBar(w.score.toFloat(), Modifier.weight(1f))
            Text("%.2f".format(w.score), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold)
        }
        Text(
            "${w.tithi} · ${w.nakshatra} · ${w.yoga} · ${w.hora} hora · ${signName(w.lagnaSign.toDouble() * 30.0 + 1.0)} rising",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        if (w.factors.isNotEmpty() || w.cautions.isNotEmpty()) HorizontalDivider(color = KshanaColors.LineSoft)
        for (f in w.factors) Text("+ $f", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
        for (c in w.cautions) Text("− $c", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    }
}

/** A thin gold bar: the score as a fraction of the full width. */
@Composable
private fun ScoreBar(score: Float, modifier: Modifier = Modifier) {
    Box(modifier.height(4.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.LineSoft)) {
        Box(Modifier.fillMaxWidth(score.coerceIn(0.002f, 1f)).fillMaxHeight().background(KshanaColors.Gold))
    }
}
