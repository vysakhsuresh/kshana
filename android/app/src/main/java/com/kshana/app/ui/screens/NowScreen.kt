package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AyanamsaChoice
import uniffi.kshana_ffi.ChartRecord
import uniffi.kshana_ffi.HouseSystem
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import kotlin.math.cos
import kotlin.math.sin

/** Default place until the location picker lands. */
private const val LAT = 9.9312
private const val LON = 76.2673
private const val PLACE = "Kochi · 9.9312°N 76.2673°E"

private val SIGN_GLYPHS = listOf("♈", "♉", "♊", "♋", "♌", "♍", "♎", "♏", "♐", "♑", "♒", "♓")

@Composable
fun NowScreen() {
    val context = LocalContext.current
    var chart by remember { mutableStateOf<ChartRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableLongStateOf(System.currentTimeMillis()) }

    // The chart is recomputed every second on a background thread; the clock ticks at ~30 Hz.
    LaunchedEffect(Unit) {
        while (true) {
            try {
                val engine = EngineProvider.get(context)
                chart = withContext(Dispatchers.Default) {
                    engine.chart(Instants.now(), LAT, LON, 0.0, HouseSystem.PLACIDUS, AyanamsaChoice.LAHIRI, false)
                }
                error = null
            } catch (e: Exception) {
                error = e.message ?: e.toString()
            }
            delay(1000)
        }
    }
    LaunchedEffect(Unit) {
        while (true) {
            tick = System.currentTimeMillis()
            delay(31)
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(horizontal = 24.dp).padding(top = 48.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.Bottom) {
            Text("KSHANA", color = KshanaColors.Gold, fontFamily = FontFamily.Serif, fontSize = 22.sp, letterSpacing = 7.sp)
            Text(PLACE.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        }
        Clock(tick)
        chart?.let { Wheel(it) } ?: Spacer(Modifier.height(300.dp))
        chart?.let { PanchangaRow(it) }
        error?.let { Text(it, color = KshanaColors.Sky, style = MaterialTheme.typography.bodySmall) }
    }
}

@Composable
private fun Clock(tickMillis: Long) {
    val local = ZonedDateTime.ofInstant(java.time.Instant.ofEpochMilli(tickMillis), ZoneId.systemDefault())
    val date = local.format(DateTimeFormatter.ofPattern("EEEE, d MMMM uuuu"))
    val hms = local.format(DateTimeFormatter.ofPattern("HH:mm:ss"))
    // microseconds within the second, from the monotonic clock so the digits keep moving
    val micro = ((System.nanoTime() / 1_000) % 1_000_000).toString().padStart(6, '0')
    Column {
        Text(date, fontFamily = FontFamily.Serif, fontSize = 20.sp, color = KshanaColors.TextSoft)
        Row(verticalAlignment = Alignment.Bottom) {
            Text(hms, style = MaterialTheme.typography.headlineLarge, color = KshanaColors.Text)
            Text(".${micro.substring(0, 3)} ${micro.substring(3)}", fontFamily = FontFamily.Monospace, fontSize = 20.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 6.dp, bottom = 6.dp))
        }
        Text("${local.zone.id} · UTC${local.offset}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

/** The sky as a wheel: Ascendant on the left, zodiac counter-clockwise, planets on the inner ring. */
@Composable
private fun Wheel(chart: ChartRecord) {
    val asc = chart.cuspsTropical[0]
    val mc = chart.cuspsTropical[9]
    Canvas(modifier = Modifier.size(320.dp)) {
        val c = Offset(size.width / 2, size.height / 2)
        val rOuter = size.minDimension / 2 - 4.dp.toPx()
        val rSign = rOuter * 0.89
        val rPlanet = rOuter * 0.70
        val rInner = rOuter * 0.59
        fun at(lon: Double, r: Float): Offset {
            val phi = Math.toRadians(180.0 + (lon - asc))
            return Offset(c.x + (r * cos(phi)).toFloat(), c.y - (r * sin(phi)).toFloat())
        }
        drawCircle(KshanaColors.Gold.copy(alpha = 0.45f), rOuter, c, style = Stroke(1.5f))
        drawCircle(KshanaColors.Gold.copy(alpha = 0.3f), rSign, c, style = Stroke(1f))
        drawCircle(KshanaColors.Gold.copy(alpha = 0.22f), rInner, c, style = Stroke(1f))
        for (i in 0 until 12) {
            drawLine(KshanaColors.Gold.copy(alpha = 0.35f), at(i * 30.0, rSign), at(i * 30.0, rOuter), 1f)
        }
        // horizon and meridian
        drawLine(KshanaColors.Sky.copy(alpha = 0.7f), at(asc, rOuter), at(asc + 180, rOuter), 1.5f)
        drawLine(KshanaColors.Sky.copy(alpha = 0.4f), at(mc, rOuter), at(mc + 180, rOuter), 1f)
        // house cusps
        for (i in 1 until 12) {
            if (i == 6 || i == 9 || i == 3) continue
            drawLine(KshanaColors.Sky.copy(alpha = 0.25f), at(chart.cuspsTropical[i], rInner), at(chart.cuspsTropical[i], rSign), 1f)
        }
        // planets as ticks (glyph text is drawn by the overlay below)
        for (b in chart.bodies) {
            val col = when (b.name) { "Sun" -> KshanaColors.Gold; "Moon" -> KshanaColors.Text; else -> KshanaColors.TextSoft }
            drawLine(KshanaColors.Gold.copy(alpha = 0.8f), at(b.tropical, rSign), at(b.tropical, rSign - 6.dp.toPx()), 1.5f)
            drawCircle(col, 4.dp.toPx(), at(b.tropical, rPlanet))
        }
        drawCircle(KshanaColors.Sky, 3.dp.toPx(), at(chart.trueNodeTropical, rPlanet), style = Stroke(1.5f))
    }
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.fillMaxWidth()) {
        Text("ASCENDANT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Text(dms(asc), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = KshanaColors.Text)
        Text("${SIGN_GLYPHS[(asc / 30).toInt() % 12]} ${signName(asc)} · MC ${dms(mc)}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
    }
}

@Composable
private fun PanchangaRow(chart: ChartRecord) {
    val p = chart.panchanga
    val moon = chart.bodies.first { it.name == "Moon" }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Tile("Tithi", "${if (p.isShukla) "Shukla" else "Krishna"} ${p.tithi}", Modifier.weight(1f))
        Tile("Nakshatra", "${p.nakshatra} ${p.pada}", Modifier.weight(1f))
        Tile("Moon", "${dms(moon.sidereal)} ${signName(moon.sidereal)}", Modifier.weight(1f))
    }
    Text(
        "${chart.provenanceEngine} · ${chart.provenanceUt1} · ayanamsa ${"%.4f".format(chart.ayanamsaDeg)}°",
        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
    )
}

@Composable
private fun Tile(label: String, value: String, modifier: Modifier = Modifier) {
    Column(modifier.padding(top = 8.dp)) {
        Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Text(value, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
    }
}

internal fun signName(lon: Double): String = listOf(
    "Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo", "Libra", "Scorpio", "Sagittarius", "Capricorn", "Aquarius", "Pisces",
)[((lon % 360 + 360) % 360 / 30).toInt() % 12]

internal fun dms(lon: Double): String {
    val inSign = (lon % 360 + 360) % 360 % 30
    val d = inSign.toInt()
    val m = ((inSign - d) * 60).toInt()
    val s = (((inSign - d) * 60 - m) * 60)
    return "%d°%02d′%05.2f″".format(d, m, s)
}
