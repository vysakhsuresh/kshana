package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.width
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Uncertainty
import com.kshana.app.engine.EngineProvider
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AspectRecord
import uniffi.kshana_ffi.ChartRecord
import uniffi.kshana_ffi.PlacementRecord
import uniffi.kshana_ffi.StabilityRecord
import java.time.format.DateTimeFormatter
import kotlin.math.cos
import kotlin.math.sin

private val BIRTH_LINE = DateTimeFormatter.ofPattern("d MMMM uuuu · HH:mm:ss")
private const val MAX_ORB_DEG = 8.0
private val MAJOR_GOLD = setOf("Conjunction", "Trine", "Sextile")
private val MAJOR_SKY = setOf("Square", "Opposition")

/** Natal (Western): the wheel with aspect lines, then the placements with stability dots. */
@Composable
fun ChartScreen(profile: BirthProfile, settings: EngineSettings) {
    val context = LocalContext.current
    var chart by remember { mutableStateOf<ChartRecord?>(null) }
    var aspects by remember { mutableStateOf<List<AspectRecord>>(emptyList()) }
    var stability by remember { mutableStateOf<StabilityRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var sidereal by remember { mutableStateOf(false) }

    LaunchedEffect(profile, settings) {
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val result = withContext(Dispatchers.Default) {
                val c = engine.chart(birth, profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, settings.topocentricMoon)
                val a = engine.aspects(birth, profile.latitude, profile.longitude, MAX_ORB_DEG)
                val s = engine.stability(
                    birth, profile.latitude, profile.longitude,
                    profile.uncertaintySeconds().toULong(), settings.houseSystem, settings.ayanamsa,
                )
                Triple(c, a, s)
            }
            chart = result.first
            aspects = result.second
            stability = result.third
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("NATAL · WESTERN", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(profile.name, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "${profile.birthLocal().format(BIRTH_LINE)} · ${profile.placeName} · ${Uncertainty.label(profile.uncertaintyLevel)}",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Chip("TROPICAL", selected = !sidereal, onClick = { sidereal = false })
            Chip("SIDEREAL", selected = sidereal, onClick = { sidereal = true })
        }
        ErrorLine(error)
        val c = chart
        if (c == null) {
            Spacer(Modifier.height(320.dp))
            if (error == null) Working("casting the sky…")
        } else {
            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) { NatalWheel(c, aspects, sidereal) }
            val asc = if (sidereal) c.cuspsSidereal[0] else c.cuspsTropical[0]
            Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.fillMaxWidth()) {
                Text("ASCENDANT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                Text(dms(asc), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = KshanaColors.Text)
                Text(signName(asc), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                stability?.let { s ->
                    Text(
                        "band ±%.2f° from %s".format(s.ascBandDeg, Uncertainty.windowText(profile.uncertaintyLevel)),
                        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Gold, modifier = Modifier.padding(top = 4.dp),
                    )
                }
            }
            SectionLabel("Placements")
            Column {
                for (b in c.bodies) PlacementRow(b, sidereal, stability)
                NodeRow("Rahu", if (sidereal) norm360(c.trueNodeTropical - c.ayanamsaDeg) else c.trueNodeTropical, c, sidereal)
                NodeRow("Ketu", if (sidereal) norm360(c.trueNodeTropical - c.ayanamsaDeg + 180.0) else norm360(c.trueNodeTropical + 180.0), c, sidereal)
            }
            if (stability != null) {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(top = 6.dp)) {
                    Box(Modifier.size(8.dp).clip(CircleShape).background(KshanaColors.Sky))
                    Spacer(Modifier.width(8.dp))
                    Text("sign or house changes within the stated window", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
            }
            SectionLabel("Aspects")
            if (aspects.isEmpty()) {
                Text("none within ${MAX_ORB_DEG.toInt()}°", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            } else {
                for (a in aspects.sortedBy { it.orb }) AspectRow(a)
            }
            Text(
                "${c.provenanceEngine} · ${c.provenanceUt1} · ayanamsa ${"%.4f".format(c.ayanamsaDeg)}°",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 12.dp),
            )
        }
    }
}

/** Ascendant on the left, zodiac counter-clockwise, cusps, planets, aspect chords. */
@Composable
private fun NatalWheel(chart: ChartRecord, aspects: List<AspectRecord>, sidereal: Boolean) {
    val cusps = if (sidereal) chart.cuspsSidereal else chart.cuspsTropical
    val asc = cusps[0]
    val mc = cusps[9]
    val measurer = rememberTextMeasurer()
    val glyphStyle = TextStyle(fontFamily = FontFamily.Serif, fontSize = 13.sp, color = KshanaColors.Text)
    val signStyle = TextStyle(fontFamily = FontFamily.Serif, fontSize = 11.sp, color = KshanaColors.Gold.copy(alpha = 0.8f))
    val lonOf = { b: PlacementRecord -> if (sidereal) b.sidereal else b.tropical }
    Canvas(modifier = Modifier.size(320.dp)) {
        val c = Offset(size.width / 2, size.height / 2)
        val rOuter = size.minDimension / 2 - 4.dp.toPx()
        val rSign = rOuter * 0.89f
        val rPlanet = rOuter * 0.72f
        val rInner = rOuter * 0.58f
        fun at(lon: Double, r: Float): Offset {
            val phi = Math.toRadians(180.0 + (lon - asc))
            return Offset(c.x + (r * cos(phi)).toFloat(), c.y - (r * sin(phi)).toFloat())
        }
        drawCircle(KshanaColors.Gold.copy(alpha = 0.45f), rOuter, c, style = Stroke(1.5f))
        drawCircle(KshanaColors.Gold.copy(alpha = 0.3f), rSign, c, style = Stroke(1f))
        drawCircle(KshanaColors.Gold.copy(alpha = 0.22f), rInner, c, style = Stroke(1f))
        for (i in 0 until 12) {
            drawLine(KshanaColors.Gold.copy(alpha = 0.35f), at(i * 30.0, rSign), at(i * 30.0, rOuter), 1f)
            val lay = measurer.measure(SIGN_SHORT[i], signStyle)
            val p = at(i * 30.0 + 15.0, (rSign + rOuter) / 2)
            drawText(lay, topLeft = Offset(p.x - lay.size.width / 2, p.y - lay.size.height / 2))
        }
        drawLine(KshanaColors.Sky.copy(alpha = 0.7f), at(asc, rOuter), at(asc + 180, rOuter), 1.5f)
        drawLine(KshanaColors.Sky.copy(alpha = 0.4f), at(mc, rOuter), at(mc + 180, rOuter), 1f)
        for (i in 1 until 12) {
            if (i == 3 || i == 6 || i == 9) continue
            drawLine(KshanaColors.Sky.copy(alpha = 0.25f), at(cusps[i], rInner), at(cusps[i], rSign), 1f)
        }
        // aspect chords inside the inner ring
        val byName = chart.bodies.associateBy { it.name }
        val dash = PathEffect.dashPathEffect(floatArrayOf(7f, 6f))
        for (a in aspects) {
            val pa = byName[a.bodyA] ?: continue
            val pb = byName[a.bodyB] ?: continue
            val colour = when (a.aspect) {
                in MAJOR_GOLD -> KshanaColors.Gold.copy(alpha = 0.75f)
                in MAJOR_SKY -> KshanaColors.Sky.copy(alpha = 0.75f)
                else -> KshanaColors.Muted.copy(alpha = 0.6f)
            }
            val minor = a.aspect !in MAJOR_GOLD && a.aspect !in MAJOR_SKY
            drawLine(colour, at(lonOf(pa), rInner), at(lonOf(pb), rInner), if (minor) 1f else 1.5f, pathEffect = if (minor) dash else null)
        }
        // planets: tick on the sign ring, glyph on the planet ring
        for (b in chart.bodies) {
            val lon = lonOf(b)
            drawLine(KshanaColors.Gold.copy(alpha = 0.8f), at(lon, rSign), at(lon, rSign - 6.dp.toPx()), 1.5f)
            drawLine(KshanaColors.Gold.copy(alpha = 0.25f), at(lon, rInner), at(lon, rInner + 4.dp.toPx()), 1f)
            val lay = measurer.measure(PLANET_GLYPHS[b.name] ?: b.name.take(2), glyphStyle)
            val p = at(lon, rPlanet)
            drawText(lay, topLeft = Offset(p.x - lay.size.width / 2, p.y - lay.size.height / 2))
        }
        val node = if (sidereal) norm360(chart.trueNodeTropical - chart.ayanamsaDeg) else chart.trueNodeTropical
        drawCircle(KshanaColors.Sky, 3.dp.toPx(), at(node, rPlanet), style = Stroke(1.5f))
    }
}

@Composable
private fun PlacementRow(b: PlacementRecord, sidereal: Boolean, stability: StabilityRecord?) {
    val lon = if (sidereal) b.sidereal else b.tropical
    val st = stability?.bodies?.firstOrNull { it.name == b.name }
    val moves = st != null && ((if (sidereal) !st.signStableSidereal else !st.signStableTropical) || !st.houseStable)
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().height(44.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(PLANET_GLYPHS[b.name] ?: "·", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = if (b.name == "Sun") KshanaColors.Gold else KshanaColors.Text, modifier = Modifier.width(28.dp))
        Text(b.name, fontFamily = FontFamily.Serif, fontSize = 16.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        Text("${degMin(lon)} ${SIGN_SHORT[signIndex(lon)]}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
        Text(if (b.speed < 0) "℞" else " ", fontFamily = FontFamily.Serif, fontSize = 12.sp, color = KshanaColors.Gold, modifier = Modifier.width(18.dp).padding(start = 4.dp))
        Text("H${b.house}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Muted, modifier = Modifier.width(34.dp).padding(start = 6.dp))
        Box(Modifier.size(8.dp).clip(CircleShape).background(if (moves) KshanaColors.Sky else KshanaColors.LineSoft))
    }
}

@Composable
private fun NodeRow(name: String, lon: Double, chart: ChartRecord, sidereal: Boolean) {
    val cusps = if (sidereal) chart.cuspsSidereal else chart.cuspsTropical
    val house = houseOf(lon, cusps)
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().height(44.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(PLANET_GLYPHS[name] ?: "·", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text, modifier = Modifier.width(28.dp))
        Text(name, fontFamily = FontFamily.Serif, fontSize = 16.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        Text("${degMin(lon)} ${SIGN_SHORT[signIndex(lon)]}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
        Spacer(Modifier.width(18.dp))
        Text("H$house", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Muted, modifier = Modifier.width(34.dp).padding(start = 6.dp))
        Box(Modifier.size(8.dp).clip(CircleShape).background(KshanaColors.LineSoft))
    }
}

@Composable
private fun AspectRow(a: AspectRecord) {
    val colour = when (a.aspect) {
        in MAJOR_GOLD -> KshanaColors.Gold
        in MAJOR_SKY -> KshanaColors.Sky
        else -> KshanaColors.Muted
    }
    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("${PLANET_GLYPHS[a.bodyA] ?: a.bodyA} ${PLANET_GLYPHS[a.bodyB] ?: a.bodyB}", fontFamily = FontFamily.Serif, fontSize = 16.sp, color = KshanaColors.Text, modifier = Modifier.width(48.dp))
        Text("${a.bodyA} ${a.aspect.lowercase()} ${a.bodyB}", style = MaterialTheme.typography.bodyMedium, color = colour, modifier = Modifier.weight(1f))
        Text(
            "%.2f° %s".format(a.orb, when (a.applying) { true -> "applying"; false -> "separating"; null -> "" }),
            fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Muted,
        )
    }
}

/** House number (1–12) of a longitude from twelve cusps, walking the zodiac forward. */
internal fun houseOf(lon: Double, cusps: List<Double>): Int {
    val x = norm360(lon)
    for (i in 0 until 12) {
        val a = norm360(cusps[i])
        val b = norm360(cusps[(i + 1) % 12])
        val inside = if (a <= b) x >= a && x < b else x >= a || x < b
        if (inside) return i + 1
    }
    return 1
}
