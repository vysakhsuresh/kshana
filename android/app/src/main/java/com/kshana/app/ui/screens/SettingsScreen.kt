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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
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
import com.kshana.app.data.ProfileStore
import com.kshana.app.data.Uncertainty
import com.kshana.app.engine.EngineProvider
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AyanamsaChoice
import uniffi.kshana_ffi.ChartRecord
import uniffi.kshana_ffi.HouseSystem
import uniffi.kshana_ffi.engineVersion

private val AYANAMSAS = listOf(
    "Lahiri" to AyanamsaChoice.LAHIRI,
    "Raman" to AyanamsaChoice.RAMAN,
    "Krishnamurti" to AyanamsaChoice.KRISHNAMURTI,
    "Fagan-Bradley" to AyanamsaChoice.FAGAN_BRADLEY,
)

private val HOUSES = listOf(
    "Placidus" to HouseSystem.PLACIDUS,
    "Whole sign" to HouseSystem.WHOLE_SIGN,
    "Equal" to HouseSystem.EQUAL,
    "Porphyry" to HouseSystem.PORPHYRY,
)

/** Profile summary, engine choices, and the provenance of the last chart cast with them. */
@Composable
fun SettingsScreen(
    profile: BirthProfile?,
    settings: EngineSettings,
    onSettingsChanged: (EngineSettings) -> Unit,
    onEditBirth: () -> Unit,
    onTrust: () -> Unit,
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    var chart by remember { mutableStateOf<ChartRecord?>(null) }
    var version by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(profile, settings) {
        try {
            val engine = EngineProvider.get(context)
            val r = withContext(Dispatchers.Default) {
                val v = engineVersion()
                val c = profile?.let {
                    engine.chart(it.birthInstant(), it.latitude, it.longitude, 0.0, settings.houseSystem, settings.ayanamsa, settings.topocentricMoon)
                }
                Pair(v, c)
            }
            version = r.first
            chart = r.second
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    fun save(s: EngineSettings) {
        ProfileStore.saveSettings(context, s)
        onSettingsChanged(s)
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("SETTINGS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("How the sky is read", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)

        SectionLabel("Birth")
        if (profile == null) {
            Text("No birth details yet.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            Text(profile.name, fontFamily = FontFamily.Serif, fontSize = 20.sp, color = KshanaColors.Text)
            Text(profile.birthLocal().format(DATE_TIME_S), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
            Text(
                "${profile.placeName} · ${"%.4f".format(profile.latitude)}° ${"%.4f".format(profile.longitude)}° · ${profile.zoneId} · time ${Uncertainty.label(profile.uncertaintyLevel)}",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }
        QuietButton(if (profile == null) "ENTER BIRTH DETAILS" else "EDIT BIRTH DETAILS", onClick = onEditBirth)

        SectionLabel("Ayanamsa")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((label, a) in AYANAMSAS) Chip(label.uppercase(), selected = settings.ayanamsa == a, onClick = { save(settings.copy(ayanamsa = a)) })
        }
        SectionLabel("House system")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((label, h) in HOUSES) Chip(label.uppercase(), selected = settings.houseSystem == h, onClick = { save(settings.copy(houseSystem = h)) })
        }
        SectionLabel("Moon")
        Row(Modifier.fillMaxWidth().height(44.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween) {
            Column(Modifier.weight(1f)) {
                Text("Topocentric Moon", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
                Text("Seen from the place rather than the Earth's centre; up to about 1° of parallax.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
            Switch(
                checked = settings.topocentricMoon,
                onCheckedChange = { save(settings.copy(topocentricMoon = it)) },
                colors = SwitchDefaults.colors(
                    checkedThumbColor = KshanaColors.Night,
                    checkedTrackColor = KshanaColors.Gold,
                    uncheckedThumbColor = KshanaColors.Muted,
                    uncheckedTrackColor = KshanaColors.Surface,
                    uncheckedBorderColor = KshanaColors.Line,
                ),
            )
        }

        SectionLabel("Provenance")
        ErrorLine(error)
        MonoRow("engine", version ?: "—")
        val c = chart
        if (c == null) {
            Text(if (profile == null) "Cast a chart to see its provenance." else "casting…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            MonoRow("chart engine", c.provenanceEngine)
            MonoRow("kernel", c.provenanceKernel)
            MonoRow("UT1", c.provenanceUt1)
            MonoRow("models", c.provenanceModels)
            MonoRow("ayanamsa at birth", "%.6f°".format(c.ayanamsaDeg))
            MonoRow("obliquity", "%.6f°".format(c.obliquityDeg))
            MonoRow("TT JD", "%.1f + %.9f".format(c.ttJd1, c.ttJd2))
        }
        Spacer(Modifier.height(8.dp))
        Text("Everything is computed on this phone. Nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)

        SectionLabel("Trust")
        Text("What the app does not know, a self-test on this phone, the error budget, and the full report to export.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        QuietButton("TRUST & VERIFICATION", onClick = onTrust)
    }
}
