package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.gestures.detectTapGestures
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.YearRowRecord
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

private val LIFE_AREAS = listOf("Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Foreign")
private val MONTH_YEAR = DateTimeFormatter.ofPattern("MMMM uuuu")
private const val MAP_YEARS = 100
private val YEAR_WIDTH = 12.dp
private val MAP_PAD = 24.dp
private val MAP_HEIGHT = 220.dp

/** A hundred years as bars: one per year, the chosen life area's score, the mahadashas marked. */
@Composable
fun LifeMapScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val density = LocalDensity.current
    val scope = rememberCoroutineScope()
    val zone = remember(profile) { profile.zone() }
    var rows by remember { mutableStateOf<List<YearRowRecord>>(emptyList()) }
    var area by remember { mutableIntStateOf(0) }
    var selectedAge by remember { mutableStateOf<Int?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(true) }
    val chipScroll = rememberScrollState()
    val mapScroll = rememberScrollState()

    LaunchedEffect(profile, settings) {
        loading = true
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            rows = withContext(Dispatchers.Default) {
                engine.lifeMap(birth, profile.latitude, profile.longitude, settings.ayanamsa, MAP_YEARS.toUInt())
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        loading = false
    }

    // The year we are in now, by the engine's own year starts.
    val currentAge = remember(rows) {
        val nowS = Instants.now().seconds()
        rows.lastOrNull { it.start.seconds() <= nowS }?.age?.toInt()
    }
    val yearPx = with(density) { YEAR_WIDTH.toPx() }
    val padPx = with(density) { MAP_PAD.toPx() }

    // First sight: the current year, a little left of centre; the current year selected.
    LaunchedEffect(rows) {
        if (rows.isEmpty()) return@LaunchedEffect
        if (selectedAge == null) selectedAge = currentAge
        val target = currentAge ?: 0
        mapScroll.scrollTo((padPx + target * yearPx - with(density) { 140.dp.toPx() }).coerceAtLeast(0f).toInt())
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("LIFE MAP", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                QuietButton("BACK", onClick = onBack)
            }
            Text("A hundred years", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Text(
                when {
                    loading -> "laying out every year from birth…"
                    rows.isEmpty() -> "The engine returned no years."
                    else -> "${rows.size} years from ${localText(rows.first().start, DATE_ONLY, zone)} · ${profile.placeName}"
                },
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            ErrorLine(error)
        }

        // Area chips, one chosen, with the arrow that says there is more to the right.
        Box(Modifier.fillMaxWidth().height(44.dp)) {
            Row(Modifier.horizontalScroll(chipScroll).padding(start = 24.dp, end = 96.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for ((i, name) in LIFE_AREAS.withIndex()) {
                    Chip(name.uppercase(), selected = i == area, onClick = { area = i })
                }
            }
            ScrollHint(chipScroll, "more areas") { scope.launch { chipScroll.animateScrollBy(with(density) { 200.dp.toPx() }) } }
        }

        if (rows.isNotEmpty()) {
            Box(Modifier.fillMaxWidth().height(MAP_HEIGHT)) {
                Box(Modifier.horizontalScroll(mapScroll)) {
                    YearBars(rows, area, currentAge, selectedAge, yearPx, padPx) { selectedAge = it }
                }
                ScrollHint(mapScroll, "later years") { scope.launch { mapScroll.animateScrollBy(with(density) { 240.dp.toPx() }) } }
            }
            Row(Modifier.padding(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                Legend(KshanaColors.Gold, "strong ≥ 0.60")
                Legend(KshanaColors.Sky, "fair 0.40–0.60")
                Legend(KshanaColors.Muted, "testing")
                Spacer(Modifier.weight(1f))
                Text("${YEAR_WIDTH.value.toInt()} dp / year", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            }

            val row = selectedAge?.let { a -> rows.firstOrNull { it.age.toInt() == a } }
            if (row != null) {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    SectionLabel(if (row.age.toInt() == currentAge) "This year" else "Chosen year")
                    Text(
                        "Age ${row.age} · ${localText(row.start, MONTH_YEAR, zone)}",
                        style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text,
                    )
                    Text("${row.maha} mahadasha, ${row.antar} antardasha", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                    val score = row.areas.getOrNull(area) ?: 0.0
                    Text(
                        "${LIFE_AREAS[area]} looks like ${stretch(score)}.",
                        fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text,
                    )
                    for ((i, name) in LIFE_AREAS.withIndex()) {
                        val v = row.areas.getOrNull(i) ?: 0.0
                        MonoRow(name, "%.2f".format(v), if (i == area) KshanaColors.Gold else scoreColour(v))
                    }
                }
            } else {
                Text("Tap a year to read it.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(horizontal = 24.dp))
            }
        }
    }
}

/** The bars: one per year, the chosen area's score; mahadasha starts lined and named; now in gold. */
@Composable
private fun YearBars(
    rows: List<YearRowRecord>,
    area: Int,
    currentAge: Int?,
    selectedAge: Int?,
    yearPx: Float,
    padPx: Float,
    onSelect: (Int) -> Unit,
) {
    val measurer = rememberTextMeasurer()
    val lordStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 10.sp, letterSpacing = 1.sp, color = KshanaColors.Gold.copy(alpha = 0.9f))
    val tickStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
    val totalWidth = MAP_PAD * 2 + YEAR_WIDTH * rows.size
    Canvas(
        Modifier.width(totalWidth).height(MAP_HEIGHT).pointerInput(rows, yearPx, padPx) {
            detectTapGestures { off ->
                val i = ((off.x - padPx) / yearPx).toInt()
                if (i in rows.indices) onSelect(rows[i].age.toInt())
            }
        },
    ) {
        val labelH = 18.dp.toPx()
        val barTop = labelH + 10.dp.toPx()
        val barBottom = size.height - 18.dp.toPx()
        val span = barBottom - barTop
        fun x(i: Int): Float = padPx + i * yearPx

        // the floor
        drawLine(KshanaColors.Line, Offset(padPx, barBottom), Offset(x(rows.size), barBottom), 1f)

        // bars
        for ((i, r) in rows.withIndex()) {
            val v = (r.areas.getOrNull(area) ?: 0.0).coerceIn(0.0, 1.0)
            val h = (v * span).toFloat().coerceAtLeast(2f)
            val left = x(i) + 1.5f
            val w = yearPx - 3f
            drawRect(scoreColour(v), topLeft = Offset(left, barBottom - h), size = Size(w, h))
            if (selectedAge != null && r.age.toInt() == selectedAge) {
                drawRect(KshanaColors.Text, topLeft = Offset(left - 1f, barBottom - h - 1f), size = Size(w + 2f, h + 2f), style = Stroke(1.5f))
            }
        }

        // mahadasha starts: a thin line and the lord's name
        for ((i, r) in rows.withIndex()) {
            if (i > 0 && rows[i - 1].maha == r.maha) continue
            val xl = x(i)
            drawLine(KshanaColors.Line, Offset(xl, labelH), Offset(xl, barBottom), 1f)
            val length = rows.drop(i).takeWhile { it.maha == r.maha }.size
            val room = length * yearPx - 4f
            var lay = measurer.measure(r.maha, lordStyle)
            if (lay.size.width > room) lay = measurer.measure(PLANET_ABBR[r.maha] ?: r.maha.take(2), lordStyle)
            if (lay.size.width <= room || i == rows.lastIndex) {
                drawText(lay, topLeft = Offset(xl + 3f, 0f))
            }
        }

        // decade ticks
        for (i in rows.indices step 10) {
            val lay = measurer.measure(rows[i].age.toString(), tickStyle)
            drawLine(KshanaColors.Line, Offset(x(i), barBottom), Offset(x(i), barBottom + 4f), 1f)
            drawText(lay, topLeft = Offset(x(i) + 2f, barBottom + 5f))
        }

        // now
        if (currentAge != null) {
            val idx = rows.indexOfFirst { it.age.toInt() == currentAge }
            if (idx >= 0) {
                val xn = x(idx) + yearPx / 2
                drawLine(KshanaColors.Gold, Offset(xn, labelH + 2f), Offset(xn, barBottom), 2f)
                val tri = Path().apply {
                    moveTo(xn - 5.dp.toPx(), labelH - 6f)
                    lineTo(xn + 5.dp.toPx(), labelH - 6f)
                    lineTo(xn, labelH + 2f)
                    close()
                }
                drawPath(tri, KshanaColors.Gold)
            }
        }
    }
}

/** A fade and a gold chevron at the right edge while there is more to scroll. */
@Composable
private fun ScrollHint(state: ScrollState, description: String, onClick: () -> Unit) {
    if (state.value >= state.maxValue) return
    Box(Modifier.fillMaxSize()) {
        Box(
            Modifier.align(Alignment.CenterEnd).width(96.dp).fillMaxHeight()
                .background(Brush.horizontalGradient(listOf(Color.Transparent, KshanaColors.Night, KshanaColors.Night))),
        )
        Box(
            Modifier.align(Alignment.CenterEnd).padding(end = 24.dp).size(44.dp).clip(CircleShape).background(KshanaColors.Gold).clickable(onClick = onClick),
            contentAlignment = Alignment.Center,
        ) {
            Icon(Icons.Filled.KeyboardArrowRight, contentDescription = description, tint = KshanaColors.Night)
        }
    }
}

@Composable
private fun Legend(colour: Color, label: String) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Box(Modifier.size(8.dp).background(colour))
        Text(label, fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
    }
}

private fun scoreColour(v: Double): Color = when {
    v >= 0.6 -> KshanaColors.Gold
    v >= 0.4 -> KshanaColors.Sky
    else -> KshanaColors.Muted
}

private fun stretch(v: Double): String = when {
    v >= 0.7 -> "a strong stretch"
    v >= 0.5 -> "a fair stretch"
    v >= 0.35 -> "a mixed stretch"
    else -> "a testing stretch"
}
