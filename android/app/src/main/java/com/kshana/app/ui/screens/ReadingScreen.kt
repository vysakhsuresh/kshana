package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
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
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.CalibrationStore
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AreaReading
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.TimingRecord
import com.kshana.app.ui.sky.KshanaFonts

private const val FUNNEL_LEVELS = 5

/** The reading for now: the strongest life area, its timing, and only the engine's own factors. */
@Composable
fun ReadingScreen(profile: BirthProfile, settings: EngineSettings, onAreas: () -> Unit, onParihara: () -> Unit) {
    val context = LocalContext.current
    var readings by remember { mutableStateOf<List<AreaReading>>(emptyList()) }
    var now by remember { mutableStateOf<ExactInstant?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(true) }
    var answersVersion by remember { mutableIntStateOf(0) }

    LaunchedEffect(profile, settings) {
        loading = true
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val at = Instants.now()
            readings = withContext(Dispatchers.Default) {
                engine.areaReadings(birth, profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, at, FUNNEL_LEVELS.toUByte())
            }
            now = at
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        loading = false
    }

    val best = remember(readings) {
        // Most dasha levels touching the houses first; then the KP net (signed, so an
        // outweighed area ranks below an unsupported one); then the Parashari score.
        readings.maxWithOrNull(compareBy<AreaReading>({ it.agreeingLevels.toInt() }, { it.promise.net }, { it.parashariScore }))
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("READING · NOW", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        ErrorLine(error)
        val r = best
        val at = now
        if (r == null || at == null) {
            if (loading) Working("judging every life area for this instant…")
            else if (error == null) Text("The engine returned no readings.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            val p = r.promise
            Text(p.area, style = MaterialTheme.typography.displayLarge, color = KshanaColors.Text)
            VerdictPair(r)
            Text(
                traditionsText(r),
                style = MaterialTheme.typography.bodyMedium, color = if (r.traditionsAgree) KshanaColors.Gold else KshanaColors.Sky,
            )
            Row(verticalAlignment = Alignment.Bottom) {
                Text("${r.agreeingLevels}/$FUNNEL_LEVELS", style = MaterialTheme.typography.headlineLarge, color = KshanaColors.Sky)
                Text(timingText(r, FUNNEL_LEVELS), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1f).padding(start = 10.dp, bottom = 10.dp))
            }

            val maha = r.funnel.firstOrNull { it.level.toInt() == 1 }
            val antar = r.funnel.firstOrNull { it.level.toInt() == 2 }
            if (maha != null) {
                SectionLabel("Timing")
                DashaTimeline(maha, antar, at)
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(localText(maha.start, DATE_ONLY), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                    Text("${maha.lord} mahadasha${antar?.let { " · ${it.lord} antar" } ?: ""}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text(localText(maha.end, DATE_ONLY), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                }
                antar?.let {
                    Text("antar ${localText(it.start, DATE_TIME)} → ${localText(it.end, DATE_TIME)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
                }
            }

            if (p.factors.isNotEmpty()) {
                SectionLabel("In short")
                Text(p.factors.take(3).joinToString(" ") { sentence(it) }, fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
            }

            SectionLabel("Why")
            if (p.factors.isEmpty()) Text("no factors recorded", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            for ((i, f) in p.factors.withIndex()) FactorRow(i, f, p.ruleId, p.source, null)
            Text(
                "cusp ${p.primaryCusp} · sub lord ${p.subLord} · ${kpNetText(p)}",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted, modifier = Modifier.padding(top = 4.dp),
            )
            val ruleIds = remember(r) { (listOf(p.ruleId) + r.parashariHits.map { it.ruleId }).distinct() }
            val calibration = remember(ruleIds, answersVersion) { ruleIds.map { it to CalibrationStore.stats(context, it) } }
            for ((id, st) in calibration) {
                if (st.second > 0) {
                    Text(
                        "rule $id · right ${st.first} of ${st.second} ${if (st.second == 1) "time" else "times"} for you",
                        fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Sky, modifier = Modifier.padding(top = 2.dp),
                    )
                }
            }

            SectionLabel("Parashari")
            Text("Parashari assessment ${"%.2f".format(r.parashariScore)}, lord ${r.parashariLord}", fontFamily = KshanaFonts.serif, fontSize = 18.sp, color = KshanaColors.Text)
            for ((i, h) in r.parashariHits.withIndex()) {
                for ((j, f) in h.factors.withIndex()) FactorRow(1000 + i * 100 + j, f, h.ruleId, h.source, h.weight)
            }
            SectionLabel("Did this happen?")
            DidItHappen(area = p.area, antarStart = antar?.start ?: maha?.start, ruleIds = ruleIds, onAnswered = { answersVersion++ })

            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                QuietButton("LIFE AREAS", onClick = onAreas, modifier = Modifier.weight(1f))
                QuietButton("PARIHARA", onClick = onParihara, modifier = Modifier.weight(1f))
            }
            Text("judged at ${localText(at, DATE_TIME_S)}${microText(at)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        }
    }
}

/** The mahadasha as a bar, the antardasha lit, a gold tick for now. */
@Composable
private fun DashaTimeline(maha: TimingRecord, antar: TimingRecord?, at: ExactInstant) {
    Canvas(Modifier.fillMaxWidth().height(24.dp)) {
        val w = size.width
        val barY = size.height / 2
        val h = 8.dp.toPx()
        drawRect(KshanaColors.LineSoft, topLeft = Offset(0f, barY - h / 2), size = Size(w, h))
        if (antar != null) {
            val a = progress(maha.start, maha.end, antar.start) * w
            val b = progress(maha.start, maha.end, antar.end) * w
            drawRect(KshanaColors.Sky.copy(alpha = 0.55f), topLeft = Offset(a, barY - h / 2), size = Size((b - a).coerceAtLeast(2f), h))
        }
        val x = progress(maha.start, maha.end, at) * w
        drawLine(KshanaColors.Gold, Offset(x, 0f), Offset(x, size.height), 2f)
    }
}

@Composable
private fun FactorRow(key: Int, factor: String, ruleId: String, source: String, weight: Double?) {
    var open by remember(key) { mutableStateOf(false) }
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable { open = !open }.padding(vertical = 8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(factor, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
            Text(if (open) "−" else "+", fontFamily = KshanaFonts.mono, fontSize = 14.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 12.dp))
        }
        if (open) {
            Text(
                "rule $ruleId · $source${weight?.let { " · weight ${"%.2f".format(it)}" } ?: ""}",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Sky, modifier = Modifier.padding(top = 4.dp),
            )
        }
    }
}

/**
 * Yes / Not yet, remembered per area and antardasha start. The same answer is also
 * recorded against every rule that supported this reading, for the calibration lines.
 */
@Composable
private fun DidItHappen(area: String, antarStart: ExactInstant?, ruleIds: List<String>, onAnswered: () -> Unit) {
    val context = LocalContext.current
    val window = "$area:${antarStart?.taiSeconds ?: 0L}"
    val key = "answer:$window"
    var answer by remember(key) { mutableIntStateOf(ProfileStore.prefs(context).getInt(key, 0)) }
    fun set(v: Int) {
        answer = v
        ProfileStore.prefs(context).edit().putInt(key, v).apply()
        for (id in ruleIds) {
            if (v == 0) CalibrationStore.clear(context, id, window)
            else CalibrationStore.record(context, id, window, happened = v == 1)
        }
        onAnswered()
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Chip("YES", selected = answer == 1, onClick = { set(if (answer == 1) 0 else 1) })
        Chip("NOT YET", selected = answer == 2, onClick = { set(if (answer == 2) 0 else 2) })
    }
    Text(
        when (answer) { 1 -> "Recorded: it happened."; 2 -> "Recorded: not yet."; else -> "Your answer stays on this phone and is kept per period." },
        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
    )
}
