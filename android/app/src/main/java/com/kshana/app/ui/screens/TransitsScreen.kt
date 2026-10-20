package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
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
import com.kshana.app.ui.sky.BreathingRing
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.KakshyaWindowRecord
import uniffi.kshana_ffi.TransitScoreRecord
import uniffi.kshana_ffi.TransitWindowRecord
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

/** The nine grahas in the engine's lane order. */
private val BODIES = listOf("Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu")
private val SIGN_NAMES = listOf(
    "Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo", "Libra", "Scorpio", "Sagittarius", "Capricorn", "Aquarius", "Pisces",
)
private val MOORTHI_METAL = mapOf("Swarna" to "gold", "Rajata" to "silver", "Tamra" to "copper", "Loha" to "iron")
private val MONTH_SHORT = DateTimeFormatter.ofPattern("MMM")
private val MONTH_YEAR_SHORT = DateTimeFormatter.ofPattern("MMM uuuu")

private const val MAX_DAYS = 800L
private const val DAY_S = 86_400.0
private const val MOON_KAKSHYA_LIMIT = 40
private val LANE_H = 28.dp
private val DAY_WIDTH = 1.2.dp
private val TL_PAD = 12.dp
private val TL_TOP = 10.dp
private val TL_AXIS = 22.dp
private val TL_HEIGHT = TL_TOP + LANE_H * BODIES.size + TL_AXIS

private enum class TransitRange(val label: String) {
    THIS_YEAR("This year"),
    NEXT_TWO("Next 2 years"),
    PAST_YEAR("Past year"),
}

/** A loaded stretch of time: exact bounds and their seconds for the x axis. */
private class Span(val from: ExactInstant, val to: ExactInstant) {
    val fromS: Double = from.seconds()
    val toS: Double = to.seconds()
    val days: Double get() = (toS - fromS) / DAY_S
}

private class MonthTick(val day: Float, val label: String, val year: Boolean)

/**
 * The sky over your chart: the nine grahas' transit windows through the sidereal signs,
 * each judged against the natal Moon and lagna, drawn as lanes on a scrolling timeline.
 */
@Composable
fun TransitsScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val density = LocalDensity.current
    val scope = rememberCoroutineScope()
    val zone = remember { ZoneId.systemDefault() }
    val now = remember(profile, settings) { Instants.now() }

    var scoreNow by remember(profile, settings) { mutableStateOf<TransitScoreRecord?>(null) }
    var scoreLoading by remember { mutableStateOf(true) }
    var scoreError by remember { mutableStateOf<String?>(null) }

    var range by remember { mutableStateOf(TransitRange.THIS_YEAR) }
    val span = remember(range, zone) { spanFor(range, zone) }
    val cache = remember(profile, settings) { mutableStateMapOf<TransitRange, List<TransitWindowRecord>>() }
    var windowsLoading by remember { mutableStateOf(false) }
    var windowsError by remember { mutableStateOf<String?>(null) }
    var selected by remember(profile, settings) { mutableStateOf<TransitWindowRecord?>(null) }

    var kakshyaFor by remember(profile, settings) { mutableStateOf<TransitWindowRecord?>(null) }
    var kakshya by remember { mutableStateOf<List<KakshyaWindowRecord>?>(null) }
    var kakshyaLoading by remember { mutableStateOf(false) }
    var kakshyaError by remember { mutableStateOf<String?>(null) }

    val chipScroll = rememberScrollState()
    val laneScroll = rememberScrollState()
    val dayPx = with(density) { DAY_WIDTH.toPx() }
    val padPx = with(density) { TL_PAD.toPx() }

    // The sky's score at this instant: nine lines, one per graha.
    LaunchedEffect(profile, settings) {
        scoreLoading = true
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            scoreNow = withContext(Dispatchers.Default) {
                engine.transitScore(birth, profile.latitude, profile.longitude, settings.ayanamsa, now)
            }
            scoreError = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            scoreError = e.message ?: e.toString()
        }
        scoreLoading = false
    }

    // The windows for the chosen range, solved once and kept.
    LaunchedEffect(profile, settings, range) {
        if (cache.containsKey(range)) return@LaunchedEffect
        windowsLoading = true
        windowsError = null
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val list = withContext(Dispatchers.Default) {
                engine.transitWindows(birth, profile.latitude, profile.longitude, settings.ayanamsa, span.from, span.to)
            }
            cache[range] = list
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            windowsError = e.message ?: e.toString()
        }
        windowsLoading = false
    }

    // Kakshya detail for the chosen window, on request.
    LaunchedEffect(kakshyaFor) {
        val w = kakshyaFor ?: return@LaunchedEffect
        kakshya = null
        kakshyaError = null
        kakshyaLoading = true
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val list = withContext(Dispatchers.Default) {
                engine.kakshyaWindows(birth, profile.latitude, profile.longitude, settings.ayanamsa, w.body, w.start, w.end)
            }
            kakshya = if (w.body == "Moon") list.take(MOON_KAKSHYA_LIMIT) else list
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            kakshyaError = e.message ?: e.toString()
        }
        kakshyaLoading = false
    }

    val windows = cache[range] ?: emptyList()
    val ticks = remember(span, zone) { monthTicks(span, zone) }

    // First sight: now a little left of centre when the range holds it, else the start.
    LaunchedEffect(windows, span) {
        if (windows.isEmpty()) return@LaunchedEffect
        val nowDay = (now.seconds() - span.fromS) / DAY_S
        val target = if (nowDay >= 0.0 && nowDay <= span.days) {
            padPx + nowDay.toFloat() * dayPx - with(density) { 140.dp.toPx() }
        } else {
            0f
        }
        laneScroll.scrollTo(target.coerceAtLeast(0f).toInt())
    }

    val error = listOfNotNull(scoreError, windowsError).distinct().joinToString(" · ").ifEmpty { null }
    val lastDay = remember(span) { ExactInstant(span.to.taiSeconds - 1L, 0u) }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("TRANSITS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                QuietButton("BACK", onClick = onBack)
            }
            Text("The sky over your chart", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Text(
                "Nine grahas through the signs, judged from your Moon and lagna · ${profile.placeName}",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            ErrorLine(error)

            SectionLabel("Now")
            NowCard(scoreNow, scoreLoading, now, zone)
        }

        // Range chips, one chosen, with the arrow that says there is more to the right.
        Box(Modifier.fillMaxWidth().height(44.dp)) {
            Row(Modifier.horizontalScroll(chipScroll).padding(start = 24.dp, end = 96.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (r in TransitRange.entries) {
                    Chip(r.label.uppercase(), selected = r == range, onClick = {
                        if (r != range) {
                            range = r
                            selected = null
                            kakshyaFor = null
                            kakshya = null
                        }
                    })
                }
            }
            LaneScrollHint(chipScroll, "more ranges") { scope.launch { chipScroll.animateScrollBy(with(density) { 200.dp.toPx() }) } }
        }

        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            SectionLabel("Windows")
            Text(
                when {
                    windowsLoading -> "Solving every ingress from ${localText(span.from, DATE_ONLY, zone)} to ${localText(lastDay, DATE_ONLY, zone)}…"
                    windows.isEmpty() && windowsError == null -> "The engine returned no windows for this range."
                    windows.isEmpty() -> "The windows could not be read."
                    else -> "${windows.size} windows · ${localText(span.from, DATE_ONLY, zone)} – ${localText(lastDay, DATE_ONLY, zone)} · ${zone.id}"
                },
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }

        if (windows.isNotEmpty()) {
            Row(Modifier.fillMaxWidth().height(TL_HEIGHT)) {
                // The lane names stay put while the years scroll.
                Column(Modifier.padding(start = 16.dp, top = TL_TOP).width(32.dp)) {
                    for (b in BODIES) {
                        Box(Modifier.height(LANE_H).fillMaxWidth(), contentAlignment = Alignment.Center) {
                            Text(
                                PLANET_GLYPHS[b] ?: b.take(2),
                                fontSize = 15.sp,
                                color = if (selected?.body == b) KshanaColors.Gold else KshanaColors.Muted,
                            )
                        }
                    }
                }
                Box(Modifier.weight(1f).fillMaxHeight()) {
                    Box(Modifier.horizontalScroll(laneScroll)) {
                        TransitLanes(windows, span, ticks, now, selected, dayPx, padPx) { w ->
                            selected = w
                            if (kakshyaFor != w) {
                                kakshyaFor = null
                                kakshya = null
                            }
                        }
                    }
                    LaneScrollHint(laneScroll, "later months") { scope.launch { laneScroll.animateScrollBy(with(density) { 240.dp.toPx() }) } }
                }
            }
            Row(Modifier.padding(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                LaneLegend(KshanaColors.Gold, "strong ≥ 0.60")
                LaneLegend(KshanaColors.Sky, "fair 0.40–0.60")
                LaneLegend(KshanaColors.Muted, "testing")
                LaneLegend(null, "vedha")
                Spacer(Modifier.weight(1f))
                Text("${DAY_WIDTH.value} dp / day", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            }

            val w = selected
            if (w != null) {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    SectionLabel("Chosen window")
                    WindowCard(
                        w = w,
                        zone = zone,
                        showKakshya = kakshyaFor == w,
                        kakshya = kakshya,
                        kakshyaLoading = kakshyaLoading,
                        kakshyaError = kakshyaError,
                        onKakshya = { kakshyaFor = w },
                    )
                }
            } else {
                Text("Tap a bar to read it.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(horizontal = 24.dp))
            }
        }

        Text(
            "Every boundary is the exact ingress instant, solved to 1 µs · Vedha per Phaladeepika 26 · Ashtakavarga per BPHS",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
            modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
        )
    }
}

/** The sky's score at this instant: a breathing ring with the number, then the nine lines. */
@Composable
private fun NowCard(record: TransitScoreRecord?, loading: Boolean, now: ExactInstant, zone: ZoneId) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        if (record == null) {
            Text(
                if (loading) "Weighing the nine grahas against your Moon…" else "The sky's score could not be read.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            return@Column
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            Box(Modifier.size(64.dp), contentAlignment = Alignment.Center) {
                BreathingRing(record.score.toFloat(), Modifier.size(64.dp), windowColour(record.score))
                Text("%.2f".format(record.score), fontFamily = KshanaFonts.mono, fontSize = 15.sp, color = KshanaColors.Gold)
            }
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text("The sky ${skyWords(record.score)} now.", fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 24.sp, color = KshanaColors.Text)
                Text(localText(now, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            }
        }
        HorizontalDivider(color = KshanaColors.LineSoft)
        for ((i, line) in record.lines.withIndex()) {
            val body = BODIES.firstOrNull { line.startsWith(it) } ?: BODIES.getOrNull(i)
            Row(verticalAlignment = Alignment.Top) {
                Text(
                    body?.let { PLANET_GLYPHS[it] } ?: "·",
                    fontSize = 14.sp, color = KshanaColors.Gold, modifier = Modifier.width(22.dp).padding(top = 1.dp),
                )
                Text(line, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
            }
        }
    }
}

/** The chosen window read in full, with the kakshya detail beneath on request. */
@Composable
private fun WindowCard(
    w: TransitWindowRecord,
    zone: ZoneId,
    showKakshya: Boolean,
    kakshya: List<KakshyaWindowRecord>?,
    kakshyaLoading: Boolean,
    kakshyaError: String?,
    onKakshya: () -> Unit,
) {
    val shape = RoundedCornerShape(8.dp)
    val sign = SIGN_NAMES.getOrNull(w.sign.toInt() % 12) ?: "sign ${w.sign}"
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text("${w.body} in $sign", style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
        Text(
            "${localText(w.start, DATE_TIME, zone)} – ${localText(w.end, DATE_TIME, zone)}",
            fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky,
        )
        Text(
            "house ${w.houseFromMoon.toInt()} from the Moon, ${w.houseFromLagna.toInt()} from the lagna",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        Text(w.verdictWords, fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = if (w.favourable) KshanaColors.Text else KshanaColors.TextSoft)
        val vedha = w.vedhaBy
        if (vedha != null) {
            Text("Vedha by $vedha", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        Text("BAV ${w.bav.toInt()}/8 · SAV ${w.sav.toInt()}/56", fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        val moorthi = w.moorthi
        if (moorthi != null) {
            val metal = MOORTHI_METAL[moorthi]
            Text(
                if (metal != null) "Moorthi · $moorthi ($metal)" else "Moorthi · $moorthi",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            Box(Modifier.size(56.dp), contentAlignment = Alignment.Center) {
                BreathingRing(w.score.toFloat(), Modifier.size(56.dp), windowColour(w.score))
                Text("%.2f".format(w.score), fontFamily = KshanaFonts.mono, fontSize = 13.sp, color = windowColour(w.score))
            }
            Text("the window's score", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        if (w.factors.isNotEmpty()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            for (f in w.factors) {
                Row(verticalAlignment = Alignment.Top) {
                    Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
                    Text(sentence(f), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                }
            }
        }
        if (!showKakshya) {
            QuietButton("KAKSHYA DETAIL", onClick = onKakshya, modifier = Modifier.fillMaxWidth())
        } else {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Text("KAKSHYA · EIGHT PARTS OF $sign".uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            when {
                kakshyaLoading -> Text("Dividing the sign into eight kakshyas…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                kakshyaError != null -> ErrorLine(kakshyaError)
                kakshya == null -> Unit
                kakshya.isEmpty() -> Text(
                    if (w.body == "Rahu" || w.body == "Ketu") "${w.body} owns no Bhinnashtakavarga, so it has no kakshya windows."
                    else "No kakshya windows in this stretch.",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                )
                else -> {
                    for (k in kakshya) KakshyaRow(k, zone)
                    if (w.body == "Moon" && kakshya.size >= MOON_KAKSHYA_LIMIT) {
                        Text("The first $MOON_KAKSHYA_LIMIT of the Moon's kakshyas.", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                    }
                }
            }
        }
    }
}

@Composable
private fun KakshyaRow(k: KakshyaWindowRecord, zone: ZoneId) {
    val colour = if (k.favourable) KshanaColors.Gold else KshanaColors.Muted
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("3°45′ of ${k.lord}", style = MaterialTheme.typography.bodyMedium, color = colour, modifier = Modifier.weight(1f))
            Text(
                "${SIGN_SHORT.getOrNull(k.sign.toInt() % 12) ?: k.sign} · k${k.kakshya.toInt() + 1} · ${if (k.favourable) "good" else "weak"}",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = colour,
            )
        }
        Text(
            "${localText(k.start, DATE_TIME, zone)} – ${localText(k.end, DATE_TIME, zone)}",
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = if (k.favourable) KshanaColors.Sky else KshanaColors.Muted,
        )
    }
}

/**
 * The lanes: one per graha, time along x at [DAY_WIDTH] per day; each window a rounded bar
 * coloured by its score, the sign's short name inside when it fits; now in gold; month ticks.
 */
@Composable
private fun TransitLanes(
    windows: List<TransitWindowRecord>,
    span: Span,
    ticks: List<MonthTick>,
    now: ExactInstant,
    selected: TransitWindowRecord?,
    dayPx: Float,
    padPx: Float,
    onSelect: (TransitWindowRecord) -> Unit,
) {
    val density = LocalDensity.current
    val measurer = rememberTextMeasurer()
    val tickStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
    val yearStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.TextSoft)
    val barDark = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Night)
    val barMuted = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
    val signDark = remember(measurer) { SIGN_SHORT.map { measurer.measure(it, barDark) } }
    val signMuted = remember(measurer) { SIGN_SHORT.map { measurer.measure(it, barMuted) } }
    val topPx = with(density) { TL_TOP.toPx() }
    val laneHPx = with(density) { LANE_H.toPx() }
    val totalWidth = TL_PAD * 2 + DAY_WIDTH * span.days.toFloat()
    val fromS = span.fromS
    val nowS = now.seconds()

    Canvas(
        Modifier.width(totalWidth).height(TL_HEIGHT).pointerInput(windows, span, dayPx, padPx) {
            detectTapGestures { off ->
                val lane = ((off.y - topPx) / laneHPx).toInt()
                if (off.y < topPx || lane !in BODIES.indices) return@detectTapGestures
                val tS = fromS + ((off.x - padPx) / dayPx).toDouble() * DAY_S
                val body = BODIES[lane]
                windows.firstOrNull { it.body == body && it.start.seconds() <= tS && tS < it.end.seconds() }?.let(onSelect)
            }
        },
    ) {
        val top = topPx
        val laneH = laneHPx
        val lanesBottom = top + laneH * BODIES.size
        val x0 = padPx
        val x1 = padPx + span.days.toFloat() * dayPx
        val radius = CornerRadius(4.dp.toPx(), 4.dp.toPx())
        fun x(s: Double): Float = padPx + ((s - fromS) / DAY_S).toFloat() * dayPx

        // lane separators and the floor
        for (i in 1 until BODIES.size) {
            val y = top + i * laneH
            drawLine(KshanaColors.LineSoft, Offset(x0, y), Offset(x1, y), 1f)
        }
        drawLine(KshanaColors.Line, Offset(x0, lanesBottom), Offset(x1, lanesBottom), 1f)

        // month ticks with labels, skipping any label that would run into the last
        var lastRight = Float.NEGATIVE_INFINITY
        for (t in ticks) {
            val xt = padPx + t.day * dayPx
            drawLine(KshanaColors.Line, Offset(xt, top), Offset(xt, lanesBottom), 1f)
            drawLine(if (t.year) KshanaColors.Muted else KshanaColors.Line, Offset(xt, lanesBottom), Offset(xt, lanesBottom + 4f), 1f)
            val lay = measurer.measure(t.label, if (t.year) yearStyle else tickStyle)
            if (xt + 2f > lastRight + 6f && xt + 2f + lay.size.width <= x1 + padPx) {
                drawText(lay, topLeft = Offset(xt + 2f, lanesBottom + 5f))
                lastRight = xt + 2f + lay.size.width
            }
        }

        // the bars
        for (w in windows) {
            val lane = BODIES.indexOf(w.body)
            if (lane < 0) continue
            val s = maxOf(w.start.seconds(), fromS)
            val e = minOf(w.end.seconds(), span.toS)
            if (e <= s) continue
            val left = x(s) + 1f
            val width = (x(e) - 1f - left).coerceAtLeast(2f)
            val barTop = top + lane * laneH + 5.dp.toPx()
            val h = laneH - 10.dp.toPx()
            val outlineOnly = !w.favourable && w.vedhaBy != null
            val colour = windowColour(w.score)
            if (outlineOnly) {
                drawRoundRect(KshanaColors.Muted, topLeft = Offset(left, barTop), size = Size(width, h), cornerRadius = radius, style = Stroke(1.dp.toPx()))
            } else {
                drawRoundRect(colour, topLeft = Offset(left, barTop), size = Size(width, h), cornerRadius = radius)
            }
            val lay = (if (outlineOnly) signMuted else signDark).getOrNull(w.sign.toInt() % 12)
            if (lay != null && lay.size.width + 8f <= width) {
                drawText(lay, topLeft = Offset(left + (width - lay.size.width) / 2f, barTop + (h - lay.size.height) / 2f))
            }
            if (selected != null && w == selected) {
                drawRoundRect(
                    KshanaColors.Text, topLeft = Offset(left - 1.5f, barTop - 1.5f), size = Size(width + 3f, h + 3f),
                    cornerRadius = radius, style = Stroke(1.5f),
                )
            }
        }

        // now
        if (nowS >= fromS && nowS <= span.toS) {
            val xn = x(nowS)
            drawLine(KshanaColors.Gold, Offset(xn, top), Offset(xn, lanesBottom), 2f)
            val tri = Path().apply {
                moveTo(xn - 5.dp.toPx(), 0f)
                lineTo(xn + 5.dp.toPx(), 0f)
                lineTo(xn, top - 1f)
                close()
            }
            drawPath(tri, KshanaColors.Gold)
        }
    }
}

/** A fade and a gold chevron at the right edge while there is more to scroll. */
@Composable
private fun LaneScrollHint(state: ScrollState, description: String, onClick: () -> Unit) {
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

/** A swatch and a word; a null colour draws the vedha outline. */
@Composable
private fun LaneLegend(colour: Color?, label: String) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        if (colour != null) {
            Box(Modifier.size(8.dp).background(colour))
        } else {
            Box(Modifier.size(8.dp).border(1.dp, KshanaColors.Muted))
        }
        Text(label, fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
    }
}

private fun windowColour(v: Double): Color = when {
    v >= 0.6 -> KshanaColors.Gold
    v >= 0.4 -> KshanaColors.Sky
    else -> KshanaColors.Muted
}

private fun skyWords(v: Double): String = when {
    v >= 0.7 -> "is strongly with you"
    v >= 0.5 -> "is fair"
    v >= 0.35 -> "is mixed"
    else -> "is testing you"
}

/** The chosen range as exact bounds in the local zone, never longer than [MAX_DAYS]. */
private fun spanFor(range: TransitRange, zone: ZoneId): Span {
    val today = LocalDate.now(zone)
    val start: LocalDate
    val end: LocalDate
    when (range) {
        TransitRange.THIS_YEAR -> {
            start = today.withDayOfYear(1)
            end = start.plusYears(1)
        }
        TransitRange.NEXT_TWO -> {
            start = today
            end = today.plusYears(2)
        }
        TransitRange.PAST_YEAR -> {
            start = today.minusYears(1)
            end = today.plusDays(1)
        }
    }
    val from = Instants.fromInstant(start.atStartOfDay(zone).toInstant())
    var to = Instants.fromInstant(end.atStartOfDay(zone).toInstant())
    val limit = MAX_DAYS * 86_400L
    if (to.taiSeconds - from.taiSeconds > limit || (to.taiSeconds - from.taiSeconds == limit && to.nanos > from.nanos)) {
        to = ExactInstant(from.taiSeconds + limit, from.nanos)
    }
    return Span(from, to)
}

/** The first of every month inside the span, as a day offset and a label; the year on January and the first tick. */
private fun monthTicks(span: Span, zone: ZoneId): List<MonthTick> {
    val out = ArrayList<MonthTick>()
    var m = Instants.toLocal(span.from, zone).toLocalDate().withDayOfMonth(1)
    while (true) {
        val s = Instants.fromInstant(m.atStartOfDay(zone).toInstant()).seconds()
        if (s > span.toS) break
        if (s >= span.fromS) {
            val year = m.monthValue == 1 || out.isEmpty()
            out += MonthTick(((s - span.fromS) / DAY_S).toFloat(), m.format(if (year) MONTH_YEAR_SHORT else MONTH_SHORT), year)
        }
        m = m.plusMonths(1)
    }
    return out
}
