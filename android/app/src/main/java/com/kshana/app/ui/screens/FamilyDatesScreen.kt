package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
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
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Lang
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.GroupSlotRecord
import uniffi.kshana_ffi.PersonInput
import uniffi.kshana_ffi.purposes

/**
 * A date for all of us: a wedding, a house-warming, a naming ceremony, a family trip.
 * Pick who it involves and what it is; the hours that suit everyone come first, with who
 * is strongest and who needs care on each.
 */
@Composable
fun FamilyDatesScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val people = remember { listOf(profile) + ProfileStore.partners(context) }
    var picked by remember { mutableStateOf(people.indices.take(2).toSet()) }
    val ps = remember { purposes() }
    var purpose by remember { mutableStateOf(ps.firstOrNull { it.key == "family" }?.key ?: ps.first().key) }
    var days by remember { mutableStateOf(60) }
    var result by remember { mutableStateOf<List<GroupSlotRecord>?>(null) }
    var run by remember { mutableStateOf(0) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val chosen = picked.sorted().map { people[it] }
    LaunchedEffect(run) {
        if (run == 0 || chosen.isEmpty()) return@LaunchedEffect
        busy = true
        try {
            val engine = EngineProvider.get(context)
            result = withContext(Dispatchers.Default) {
                engine.bestSlotsTogether(
                    chosen.map { PersonInput(it.birthInstant(), it.latitude, it.longitude) },
                    place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, purpose,
                    Instants.now(), days.toUInt(), 8u, Lang.code,
                )
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        busy = false
    }
    fun first(p: BirthProfile) = p.name.substringBefore(' ').lowercase().replaceFirstChar { it.uppercase() }.ifBlank { "You" }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("A DATE FOR ALL OF US", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("When suits everyone?", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("A wedding, a house-warming, a naming ceremony, a trip, a big talk. Pick who it involves and what it is.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        SectionLabel("Who")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((i, p) in people.withIndex()) {
                OptionBox(if (i == 0) "${first(p)} (you)" else first(p), selected = i in picked, onClick = { picked = if (i in picked) picked - i else picked + i; result = null })
            }
        }
        if (people.size < 2) Text("Add people under Us to plan for more than yourself.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        SectionLabel("What")
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            for (row in ps.chunked(2)) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (p in row) OptionBox(tr(p.words), selected = purpose == p.key, onClick = { purpose = p.key; result = null }, modifier = Modifier.weight(1f))
                if (row.size == 1) Spacer(Modifier.weight(1f))
            }
        }
        SectionLabel("Within")
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (d in listOf(30, 60, 90, 180)) Chip("$d DAYS", selected = days == d, onClick = { days = d; result = null })
        }
        GoldButton(if (busy) "LOOKING AT EVERY DAY…" else "FIND THE BEST DATES", enabled = !busy && chosen.isNotEmpty(), onClick = { run++ }, modifier = Modifier.fillMaxWidth())
        ErrorLine(error)
        val r = result
        if (busy) Working("weighing every hour for ${chosen.size} people…")
        if (r != null) {
            if (r.isEmpty()) Text("No hour in this span suits everyone. Try a longer span or fewer people.", style = MaterialTheme.typography.bodyLarge, color = KshanaColors.TextSoft)
            for ((rank, s) in r.withIndex()) {
                Column(Modifier.fillMaxWidth().glass(14.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(if (rank == 0) "BEST" else "#${rank + 1}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.width(48.dp))
                        Text("${localText(s.start, WEEKDAY_DAY_YEAR)} · ${hm(s.start)} – ${hm(s.end)}", fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = KshanaColors.Sky, modifier = Modifier.weight(1f))
                        Text(s.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = scoreColor(s.score))
                    }
                    if (s.words.isNotBlank()) Text(s.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                    Text(
                        chosen.withIndex().joinToString(" · ") { (i, p) -> "${first(p)} ${s.each.getOrNull(i)?.toInt() ?: "–"}" },
                        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                    )
                    Text(
                        "ADD TO CALENDAR ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                        modifier = Modifier.clickable {
                            addToCalendar(context, "Kshana · ${ps.firstOrNull { it.key == purpose }?.words ?: ""}", Instants.toLocal(s.start).toInstant().toEpochMilli(), Instants.toLocal(s.end).toInstant().toEpochMilli(), s.words)
                        }.padding(vertical = 4.dp),
                    )
                }
            }
        }
    }
}
