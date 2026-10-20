package com.kshana.app.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AyanamsaChoice
import uniffi.kshana_ffi.EventRecord
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

private val DAY = DateTimeFormatter.ofPattern("EEE d MMM uuuu")
private val TIME = DateTimeFormatter.ofPattern("HH:mm:ss")

@Composable
fun MomentsScreen(ayanamsa: AyanamsaChoice = AyanamsaChoice.LAHIRI, onMuhurta: () -> Unit = {}) {
    val context = LocalContext.current
    var events by remember { mutableStateOf<List<EventRecord>>(emptyList()) }
    var status by remember { mutableStateOf("solving the next 14 days to the microsecond…") }

    LaunchedEffect(ayanamsa) {
        try {
            val engine = EngineProvider.get(context)
            events = withContext(Dispatchers.Default) { engine.nextEvents(Instants.now(), 14u, ayanamsa) }
            status = "${events.size} exact instants · each solved to 1 µs"
        } catch (e: Exception) {
            status = e.message ?: e.toString()
        }
    }

    Column(Modifier.fillMaxSize().padding(horizontal = 24.dp).padding(top = 48.dp)) {
        Text("EXACT MOMENTS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text("The sky is about to change", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(status, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(bottom = 12.dp))
        LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(0.dp)) {
            items(events) { e -> EventRow(e) }
        }
        QuietButton("FIND A TIME", onClick = onMuhurta, modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp))
    }
}

@Composable
private fun EventRow(e: EventRecord) {
    val local = Instants.toLocal(e.instant)
    val micro = (e.instant.nanos.toLong() / 1_000).toString().padStart(6, '0')
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().padding(vertical = 10.dp), horizontalArrangement = Arrangement.SpaceBetween) {
        Column(Modifier.weight(1f)) {
            Text(local.format(DAY), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            Text(title(e), fontFamily = KshanaFonts.serif, fontSize = 18.sp, color = KshanaColors.Text)
            Text(detail(e), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            RemindButton(
                key = "moment:${e.kind}:${e.body}:${e.instant.taiSeconds}",
                atMillis = local.toInstant().toEpochMilli() - 5L * 60L * 1000L,
                title = "${title(e)} in 5 minutes",
                text = "exact at ${local.format(DATE_TIME_S)}",
                modifier = Modifier.padding(top = 6.dp),
            )
        }
        Column(horizontalAlignment = androidx.compose.ui.Alignment.End) {
            Text(local.format(TIME), fontFamily = KshanaFonts.mono, fontSize = 13.sp, color = KshanaColors.Sky)
            Text(".${micro.substring(0, 3)} ${micro.substring(3)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold)
        }
    }
}

private fun title(e: EventRecord): String = when (e.kind) {
    "ingress" -> "${e.body} changes sign"
    "station" -> "${e.body} stations"
    "new_moon" -> "New Moon · Amavasya"
    "full_moon" -> "Full Moon · Purnima"
    "tithi" -> "Tithi changes"
    "nakshatra" -> "Moon changes nakshatra"
    else -> e.kind
}

private fun detail(e: EventRecord): String = when (e.kind) {
    "ingress" -> "tropical ingress · exact to ±${e.halfWidthNs} ns"
    "station" -> "longitude speed crosses zero"
    "new_moon", "full_moon" -> "Moon − Sun elongation exact"
    "tithi" -> "Moon − Sun crosses a 12° boundary"
    "nakshatra" -> "sidereal 13°20′ boundary · Lahiri"
    else -> ""
}
