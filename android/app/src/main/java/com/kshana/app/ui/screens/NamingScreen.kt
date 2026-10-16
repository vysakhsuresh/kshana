package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.FamilyStore
import com.kshana.app.data.ProfileStore
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.engine.EngineProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.NamingRecord
import uniffi.kshana_ffi.naming

private data class NamingResult(val record: NamingRecord, val moonSidereal: Double)

/** The traditional first syllable of a name, from the Moon's nakshatra and pada at birth. */
@Composable
fun NamingScreen(settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val people = remember { FamilyStore.everyone(context) }
    var selected by remember { mutableStateOf(people.firstOrNull()) }
    var fresh by remember { mutableStateOf(false) }
    var result by remember { mutableStateOf<NamingResult?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }

    LaunchedEffect(selected, settings) {
        val p = selected ?: return@LaunchedEffect
        busy = true
        try {
            val engine = EngineProvider.get(context)
            result = withContext(Dispatchers.Default) {
                val chart = engine.chart(p.birthInstant(), p.latitude, p.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
                val moon = chart.panchanga.moonSidereal
                NamingResult(naming(moon), moon)
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        busy = false
    }

    if (fresh) {
        val previous = remember { ProfileStore.load(context) }
        Column(Modifier.fillMaxSize()) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 24.dp).padding(top = 48.dp), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("NAMING · NEW BIRTH", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text("CANCEL", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable { fresh = false }.padding(12.dp))
            }
            BirthScreen(existing = null, onCast = { p ->
                // The form saves what it casts; put the active profile back. Nothing else is kept.
                if (previous != null) ProfileStore.save(context, previous)
                selected = p
                fresh = false
            })
        }
        return
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("NAMING", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("The first sound of a name", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)

        SectionLabel("Whose birth")
        if (people.isEmpty()) {
            Text("No one is stored yet. Enter a birth below; it is used once and not saved.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (p in people) Chip(p.name.uppercase(), selected = selected === p, onClick = { selected = p })
            }
        }
        HorizontalDivider(color = KshanaColors.LineSoft)
        Row(
            Modifier.fillMaxWidth().heightIn(min = 56.dp).clickable { fresh = true }.padding(vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text("New birth", style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                Text("A newborn, or anyone not stored. Used once, not saved.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = KshanaColors.Gold, modifier = Modifier.padding(start = 12.dp))
        }
        HorizontalDivider(color = KshanaColors.LineSoft)

        ErrorLine(error)
        val p = selected
        val r = result
        if (p != null) {
            SectionLabel(p.name)
            Text("${p.birthLocal().format(DATE_TIME)} · ${p.placeName}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Muted)
            if (r == null) {
                if (busy || error == null) Text("finding the Moon…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            } else {
                Spacer(Modifier.height(8.dp))
                Text(r.record.syllable, fontFamily = FontFamily.Serif, fontSize = 88.sp, lineHeight = 92.sp, color = KshanaColors.Gold)
                Text("Moon in ${r.record.nakshatra}, pada ${r.record.pada}", style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                MonoRow("Moon sign", signName(r.moonSidereal))
                MonoRow("Moon longitude", "${signName(r.moonSidereal)} ${degMin(r.moonSidereal)}")
                MonoRow("nakshatra", r.record.nakshatra)
                MonoRow("pada", r.record.pada.toString())
                Spacer(Modifier.height(4.dp))
                Text(
                    "Indian naming follows the Moon's star at birth: each of the twenty-seven nakshatras is divided into four padas, and each pada has its own sound. " +
                        "The syllable above is the traditional first sound of the name for this Moon; the rest of the name is the family's to choose.",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                )
            }
        }
    }
}
