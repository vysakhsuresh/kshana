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
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.FamilyStore
import com.kshana.app.data.ProfileStore
import com.kshana.app.data.Uncertainty
import com.kshana.app.engine.EngineProvider
import com.kshana.app.net.updateEop
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AyanamsaChoice
import uniffi.kshana_ffi.ChartRecord
import uniffi.kshana_ffi.HouseSystem
import uniffi.kshana_ffi.engineVersion
import java.time.Instant
import java.time.ZoneId
import com.kshana.app.notify.Reminders
import com.kshana.app.ui.sky.Haptics
import com.kshana.app.ui.sky.Motion
import com.kshana.app.ui.sky.SkySound
import uniffi.kshana_ffi.setRiseConvention
import kotlinx.coroutines.delay
import com.kshana.app.ui.sky.KshanaFonts

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

/** Profile summary, engine choices, the provenance of the last chart cast with them, and the optional online features. */
@Composable
fun SettingsScreen(
    profile: BirthProfile?,
    settings: EngineSettings,
    onSettingsChanged: (EngineSettings) -> Unit,
    onEditBirth: () -> Unit,
    onRectify: () -> Unit,
    onTrust: () -> Unit,
    onNarrate: () -> Unit,
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var chart by remember { mutableStateOf<ChartRecord?>(null) }
    var version by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    // Online section state.
    var onlineOn by remember { mutableStateOf(FamilyStore.onlineAllowed(context)) }
    var eopFetchedAt by remember { mutableLongStateOf(FamilyStore.eopFetchedAt(context)) }
    var eopUpdating by remember { mutableStateOf(false) }
    var eopMessage by remember { mutableStateOf<String?>(null) }
    var eopError by remember { mutableStateOf<String?>(null) }
    var aiKey by remember { mutableStateOf(FamilyStore.aiKey(context) ?: "") }

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
        runCatching { setRiseConvention(s.riseConvention) }
        onSettingsChanged(s)
    }

    fun setOnline(on: Boolean) {
        FamilyStore.saveOnlineAllowed(context, on)
        onlineOn = on
    }

    fun runEopUpdate() {
        if (eopUpdating) return
        scope.launch {
            eopUpdating = true
            eopMessage = null
            eopError = null
            updateEop(context)
                .onSuccess { at ->
                    eopFetchedAt = at
                    eopMessage = "Updated. The engine will use the new data from the next calculation."
                }
                .onFailure { e -> eopError = e.message ?: e.toString() }
            eopUpdating = false
        }
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
            Text(profile.name, fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text)
            Text(profile.birthLocal().format(DATE_TIME_S), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
            Text(
                "${profile.placeName} · ${"%.4f".format(profile.latitude)}° ${"%.4f".format(profile.longitude)}° · ${profile.zoneId} · time ${Uncertainty.label(profile.uncertaintyLevel)}",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }
        QuietButton(if (profile == null) "ENTER BIRTH DETAILS" else "EDIT BIRTH DETAILS", onClick = onEditBirth)
        Text("Unsure of the minute? Score every candidate time in your window against dated life events.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        QuietButton("RECTIFY FROM LIFE EVENTS", onClick = onRectify)

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

        SectionLabel("Sunrise")
        Text(
            "Hindu almanacs take sunrise when the Sun's centre reaches the true horizon; Western tables take the refracted upper limb, a few minutes earlier. Every ghati, hora and choghadiya follows the choice.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Chip("HINDU", selected = settings.riseConvention == "hindu", onClick = { save(settings.copy(riseConvention = "hindu")) })
            Chip("WESTERN", selected = settings.riseConvention == "western", onClick = { save(settings.copy(riseConvention = "western")) })
        }

        SectionLabel("Feel")
        var stillSky by remember { mutableStateOf(Motion.stillSky(context)) }
        var haptics by remember { mutableStateOf(Haptics.enabled(context)) }
        var sound by remember { mutableStateOf(SkySound.enabled(context)) }
        FeelRow("Still sky", "No breathing, drifting or parallax; the sky is drawn once and stays.", stillSky) { stillSky = it; Motion.setStillSky(context, it) }
        FeelRow("Haptics", "A felt tick at each exact instant and on every choice.", haptics) { haptics = it; Haptics.setEnabled(context, it) }
        FeelRow("Sound", "A quiet drone that follows the daylight and a bell at each exact instant. Off unless you ask.", sound) { sound = it; SkySound.setEnabled(context, it) }

        SectionLabel("Reminders")
        var exactAlarms by remember { mutableStateOf(Reminders.exactAllowed(context)) }
        LaunchedEffect(Unit) {
            while (true) {
                delay(1000)
                exactAlarms = Reminders.exactAllowed(context)
            }
        }
        if (exactAlarms) {
            Text("Reminders fire at the exact second.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        } else {
            Text(
                "Android is holding this app's reminders to an inexact slot, which can be minutes late. Allow exact alarms so each one fires at its second.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            QuietButton("ALLOW EXACT ALARMS", onClick = { Reminders.requestExact(context) })
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
        Text(
            if (onlineOn) "Everything is computed on this phone. The network is used only for the online features below, when you ask."
            else "Everything is computed on this phone. Nothing leaves it.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )

        SectionLabel("Online")
        Text(
            "Off by default. When on, you can search any place on Earth, refresh the Earth-rotation data, and ask for a spoken-style narration with your own key.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Chip("OFF", selected = !onlineOn, onClick = { setOnline(false) })
            Chip("ON", selected = onlineOn, onClick = { setOnline(true) })
        }
        if (onlineOn) {
            Spacer(Modifier.height(4.dp))
            Text("Earth rotation data (IERS)", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            MonoRow("finals2000A.all", if (eopFetchedAt > 0L) "last updated " + Instant.ofEpochMilli(eopFetchedAt).atZone(ZoneId.systemDefault()).format(DATE_ONLY) else "bundled copy")
            Text(
                "UT1−UTC and polar motion for recent dates. The bundled file ages; a fresh one tightens the error budget.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
            if (eopUpdating) {
                Text("Updating…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Sky)
            } else {
                QuietButton("UPDATE NOW", onClick = { runEopUpdate() })
            }
            eopMessage?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted) }
            ErrorLine(eopError)

            Spacer(Modifier.height(4.dp))
            Text("Narration key", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            OutlinedTextField(
                value = aiKey,
                onValueChange = {
                    aiKey = it
                    FamilyStore.saveAiKey(context, it)
                },
                label = { Text("Claude API key", style = MaterialTheme.typography.labelSmall) },
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                textStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 13.sp, color = KshanaColors.Text),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedTextColor = KshanaColors.Text,
                    unfocusedTextColor = KshanaColors.Text,
                    cursorColor = KshanaColors.Gold,
                    focusedBorderColor = KshanaColors.Gold,
                    unfocusedBorderColor = KshanaColors.Line,
                    focusedLabelColor = KshanaColors.Gold,
                    unfocusedLabelColor = KshanaColors.Muted,
                    focusedContainerColor = KshanaColors.Surface,
                    unfocusedContainerColor = KshanaColors.Surface,
                ),
                modifier = Modifier.fillMaxWidth(),
            )
            Text(
                "The key is stored only on this phone and used only when you tap Narrate. It is sent to the narration service and nowhere else.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
            QuietButton("OPEN NARRATION", onClick = onNarrate)
        }

        SectionLabel("Trust")
        Text("What the app does not know, a self-test on this phone, the error budget, and the full report to export.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        QuietButton("TRUST & VERIFICATION", onClick = onTrust)
    }
}

/** A labelled switch row in the house colours. */
@Composable
private fun FeelRow(title: String, description: String, checked: Boolean, onChecked: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            Text(description, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        Switch(
            checked = checked,
            onCheckedChange = onChecked,
            colors = SwitchDefaults.colors(
                checkedThumbColor = KshanaColors.Night,
                checkedTrackColor = KshanaColors.Gold,
                uncheckedThumbColor = KshanaColors.Muted,
                uncheckedTrackColor = KshanaColors.Surface,
                uncheckedBorderColor = KshanaColors.Line,
            ),
        )
    }
}
