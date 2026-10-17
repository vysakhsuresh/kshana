package com.kshana.app.ui.screens

import android.content.Intent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
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
import com.kshana.app.data.CalibrationStore
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Uncertainty
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.BudgetRow
import uniffi.kshana_ffi.ChartRecord
import uniffi.kshana_ffi.CheckRecord
import uniffi.kshana_ffi.SelfTestRecord
import uniffi.kshana_ffi.StabilityRecord
import uniffi.kshana_ffi.engineVersion
import uniffi.kshana_ffi.errorBudget

/** What the screen computes once, off the main thread, for the profile and settings. */
private data class TrustLoad(val version: String, val budget: List<BudgetRow>, val chart: ChartRecord?, val stability: StabilityRecord?)

/**
 * Trust and verification: what is not known, the self-test on this phone, the error
 * budget, provenance, the rules, and the full report to export.
 */
@Composable
fun TrustScreen(profile: BirthProfile?, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var load by remember { mutableStateOf<TrustLoad?>(null) }
    var selfTest by remember { mutableStateOf<SelfTestRecord?>(null) }
    var testing by remember { mutableStateOf(false) }
    var exporting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var testError by remember { mutableStateOf<String?>(null) }
    var exportError by remember { mutableStateOf<String?>(null) }
    val calibration = remember { CalibrationStore.summary(context) }

    LaunchedEffect(profile, settings) {
        try {
            val engine = EngineProvider.get(context)
            load = withContext(Dispatchers.Default) {
                val c = profile?.let {
                    engine.chart(it.birthInstant(), it.latitude, it.longitude, 0.0, settings.houseSystem, settings.ayanamsa, settings.topocentricMoon)
                }
                val s = profile?.let {
                    engine.stability(it.birthInstant(), it.latitude, it.longitude, it.uncertaintySeconds().toULong(), settings.houseSystem, settings.ayanamsa)
                }
                TrustLoad(engineVersion(), errorBudget(), c, s)
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    suspend fun runSelfTest() {
        testing = true
        try {
            val engine = EngineProvider.get(context)
            selfTest = withContext(Dispatchers.Default) { engine.selfTest() }
            testError = null
        } catch (e: Exception) {
            testError = e.message ?: e.toString()
        }
        testing = false
    }

    // The self-test runs by itself on first open; RUN repeats it.
    LaunchedEffect(Unit) { runSelfTest() }

    fun export(p: BirthProfile) {
        if (exporting) return
        scope.launch {
            exporting = true
            try {
                val engine = EngineProvider.get(context)
                val at = Instants.now()
                val json = withContext(Dispatchers.Default) {
                    engine.readingReportJson(p.birthInstant(), p.latitude, p.longitude, settings.houseSystem, settings.ayanamsa, at)
                }
                val send = Intent(Intent.ACTION_SEND).apply {
                    type = "application/json"
                    putExtra(Intent.EXTRA_SUBJECT, "Kshana reading report")
                    putExtra(Intent.EXTRA_TEXT, json)
                }
                context.startActivity(Intent.createChooser(send, "Export this reading"))
                exportError = null
            } catch (e: Exception) {
                exportError = e.message ?: e.toString()
            }
            exporting = false
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("TRUST", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("What can be checked", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        ErrorLine(error)

        SectionLabel("What this app does not know")
        if (profile == null) {
            Text("No birth details yet, so nothing is claimed about a chart.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            val level = profile.uncertaintyLevel
            Text("Birth time stated as ${Uncertainty.label(level)}.", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
            val s = load?.stability
            if (s != null) {
                Text(
                    if (level == 0) "With an exact time the ascendant is taken as computed, ±%.2f° for the engine's own error.".format(s.ascBandDeg)
                    else "Within %s the ascendant can move ±%.2f°; placements and cusps that cross a boundary in that band are marked unstable rather than asserted.".format(Uncertainty.windowText(level), s.ascBandDeg),
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                )
                val unstable = s.cuspsStable.count { !it }
                MonoRow("ascendant band", "±%.2f°".format(s.ascBandDeg))
                MonoRow("cusps that may change sign", unstable.toString(), if (unstable == 0) KshanaColors.Sky else KshanaColors.Gold)
            } else if (error == null) {
                Working("measuring the band…")
            }
            Text(
                "The engine does not know your birth time better than you do. A rule that fires is a statement in a tradition, not a measurement.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }

        SectionLabel("Self-test on this phone")
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(
                when {
                    testing -> "running…"
                    selfTest == null -> "not run yet"
                    selfTest?.allPass == true -> "every check passes"
                    else -> "a check failed"
                },
                style = MaterialTheme.typography.bodyMedium,
                color = if (selfTest?.allPass == false) KshanaColors.Sky else KshanaColors.TextSoft,
            )
            QuietButton(if (testing) "RUNNING" else "RUN", onClick = { if (!testing) scope.launch { runSelfTest() } })
        }
        ErrorLine(testError)
        selfTest?.let { t ->
            for (c in t.checks) CheckRow(c)
            HorizontalDivider(color = KshanaColors.LineSoft)
            Text(
                if (t.allPass) "${t.checks.size} of ${t.checks.size} within tolerance." else "${t.checks.count { it.pass }} of ${t.checks.size} within tolerance.",
                fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = if (t.allPass) KshanaColors.Gold else KshanaColors.Sky,
            )
        }

        SectionLabel("Error budget")
        val budget = load?.budget
        if (budget == null) {
            if (error == null) Working("reading…")
        } else {
            for (b in budget) BudgetLines(b)
        }

        SectionLabel("Provenance")
        MonoRow("engine", load?.version ?: "—")
        val c = load?.chart
        if (c == null) {
            Text(if (profile == null) "Cast a chart to see its provenance." else "casting…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            MonoRow("chart engine", c.provenanceEngine)
            MonoRow("kernel", c.provenanceKernel)
            MonoRow("UT1", c.provenanceUt1)
            MonoRow("models", c.provenanceModels)
        }

        SectionLabel("Your record")
        val record = remember { uniffi.kshana_ffi.calibrate(com.kshana.app.data.PredictionStore.records(context)) }
        Text(record.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
        for (l in record.lives.filter { it.n > 0u }) {
            Row(Modifier.fillMaxWidth().padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("${lifeGlyph(l.life)} ${l.life}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                Text("${l.hits} / ${l.n}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
                if (l.offset != 0.0) Text("  ${signedPoints(l.offset)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Gold)
            }
        }
        Text("Every window the app names on a life page can be judged after it passes. Five answers for a life make a record; a record moves nothing silently, it is shown beside the score.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)

        SectionLabel("Rules")
        Text("Every sentence traces to a rule. docs/RULES.md in the repository lists them.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        Text(calibration, fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)

        SectionLabel("Export")
        if (profile == null) {
            Text("Enter birth details to export a reading.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            Text("The whole reading as JSON: every input, every number, every rule that fired, and the provenance.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            ErrorLine(exportError)
            GoldButton(if (exporting) "PREPARING…" else "EXPORT THIS READING", enabled = !exporting, onClick = { export(profile) }, modifier = Modifier.fillMaxWidth())
        }

        Spacer(Modifier.height(12.dp))
        Text(
            "Nothing is computed on a server. Nothing is hidden: the numbers, the rules and their sources are all in this report.",
            fontFamily = FontFamily.Serif, fontStyle = FontStyle.Italic, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text,
        )
    }
}

/** One self-test check: name, expected vs got in mono, a gold dot for pass or FAIL in sky. */
@Composable
private fun CheckRow(c: CheckRecord) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(c.name, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            Text(
                "expected ${numText(c.expected)} · got ${numText(c.got)} · tol ${numText(c.tolerance)} ${c.unit}",
                fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted,
            )
        }
        if (c.pass) {
            Box(Modifier.padding(start = 12.dp).size(8.dp).clip(CircleShape).background(KshanaColors.Gold))
        } else {
            Text("FAIL", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.padding(start = 12.dp))
        }
    }
}

/** One error-budget row as three lines: layer in serif, method muted, error in mono sky. */
@Composable
private fun BudgetLines(b: BudgetRow) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp)) {
        Text(b.layer, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
        Text(b.method, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        Text(b.error, fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
    }
}

/** Enough digits to see the comparison, no trailing noise. */
private fun numText(x: Double): String {
    val s = "%.6f".format(x).trimEnd('0').trimEnd('.')
    return if (s.isEmpty() || s == "-0") "0" else s
}
