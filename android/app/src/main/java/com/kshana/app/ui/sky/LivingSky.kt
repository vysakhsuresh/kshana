package com.kshana.app.ui.sky

import androidx.compose.animation.core.EaseInOut
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ChartRecord
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.PanchangaDetail
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.math.sqrt

/*
 * The living sky: a full-bleed night canvas drawn, offline, from the real sky over a place.
 *
 * Geometry. The horizon is a line at 80 % of the height; the ecliptic is an ellipse centred
 * on it, so its upper half is the sky and its lower half lies under the ground. Looking
 * south (north of the equator) east is on the left, so the Ascendant sits at the left end
 * of the ring on the horizon, the Descendant at the right end, and a body's place along the
 * ring is the longitude distance back from the Ascendant: d = Asc − λ, 0° at the east
 * horizon, 90° at the top, 180° at the west horizon, beyond that under the ground. South of
 * the equator the picture is mirrored (east on the right). Everything is tropical, like the
 * wheel on the Now screen: the ring's sign ticks, the Ascendant from cuspsTropical[0] and
 * the bodies at their tropical longitudes.
 *
 * Drawing. All geometry is precomputed in a `drawWithCache` block (re-run when the size or
 * the minute's chart changes) into float arrays, brushes and text layouts; the per-frame
 * `onDrawBehind` only reads the twinkle and drift clocks and issues draw calls, so it
 * allocates nothing. Gradient brushes cache their shader against the canvas size.
 */

private const val STAR_COUNT = 450
private const val MILKY_WAY_BLOBS = 40
private const val STAR_SEED = 0x4B5348414E41L // "KSHANA": the same sky on every launch
private const val DRIFT_PX_PER_MINUTE = 0.6
private const val TWO_PI = 6.2831855f
private const val HORIZON_FRACTION = 0.80f
private const val TWILIGHT_SECONDS = 2400.0 // 40 minutes either side of sunrise and sunset
private const val REFRESH_MILLIS = 60_000L
private val DAY_BLUE = Color(0xFF0E1A3A)

/** Local copy: the screens' glyph map is package-private to ui.screens. */
private val PLANET_GLYPHS: Map<String, String> = mapOf(
    "Sun" to "☉", "Moon" to "☽", "Mercury" to "☿", "Venus" to "♀", "Mars" to "♂",
    "Jupiter" to "♃", "Saturn" to "♄", "Uranus" to "♅", "Neptune" to "♆", "Pluto" to "♇",
    "Rahu" to "☊", "Ketu" to "☋",
)
private val SIGN_GLYPHS = listOf("♈", "♉", "♊", "♋", "♌", "♍", "♎", "♏", "♐", "♑", "♒", "♓")
private val SIGN_NAMES = listOf(
    "Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo", "Libra", "Scorpio", "Sagittarius", "Capricorn", "Aquarius", "Pisces",
)
/** The naked-eye planets; the Sun and Moon are drawn as discs. */
private val VISIBLE_PLANETS = setOf("Mercury", "Venus", "Mars", "Jupiter", "Saturn")

private fun norm360(x: Double): Double = (x % 360.0 + 360.0) % 360.0
private fun signIndex(lon: Double): Int = (norm360(lon) / 30.0).toInt() % 12
private fun signName(lon: Double): String = SIGN_NAMES[signIndex(lon)]

/** The phase name for a Moon − Sun elongation in degrees (0 new, 180 full). */
private fun phaseName(elongationDeg: Double): String {
    val e = norm360(elongationDeg)
    return when {
        e < 22.5 || e >= 337.5 -> "new"
        e < 67.5 -> "waxing crescent"
        e < 112.5 -> "first quarter"
        e < 157.5 -> "waxing gibbous"
        e < 202.5 -> "full"
        e < 247.5 -> "waning gibbous"
        e < 292.5 -> "last quarter"
        else -> "waning crescent"
    }
}

/** A seeded linear congruential generator (Knuth's MMIX constants). */
private class Lcg(private var state: Long) {
    /** The next value in [0, 1). */
    fun next(): Float {
        state = state * 6364136223846793005L + 1442695040888963407L
        return ((state ushr 40).toInt() and 0xFFFFFF) / 16777216f
    }
}

/** The star catalogue in unit coordinates, generated once from the fixed seed. */
private class StarField(count: Int) {
    val x = FloatArray(count)
    val y = FloatArray(count)
    /** Radius in device pixels, 0.4–1.6, most stars small. */
    val radius = FloatArray(count)
    val brightness = FloatArray(count)
    /** Twinkle phase in radians. */
    val phase = FloatArray(count)
    /** 0 warm white (Text), 1 blue (Sky), 2 gold. */
    val tint = IntArray(count)
    val bandU = FloatArray(MILKY_WAY_BLOBS)
    val bandV = FloatArray(MILKY_WAY_BLOBS)
    val bandRadius = FloatArray(MILKY_WAY_BLOBS)
    val bandAlpha = FloatArray(MILKY_WAY_BLOBS)
    val bandTint = IntArray(MILKY_WAY_BLOBS)

    init {
        val rng = Lcg(STAR_SEED)
        for (i in 0 until count) {
            x[i] = rng.next()
            y[i] = rng.next()
            val r = rng.next()
            radius[i] = 0.4f + 1.2f * r * r
            brightness[i] = 0.35f + 0.65f * rng.next()
            phase[i] = rng.next() * TWO_PI
            val t = rng.next()
            tint[i] = when {
                t < 0.80f -> 0
                t < 0.93f -> 1
                else -> 2
            }
        }
        for (i in 0 until MILKY_WAY_BLOBS) {
            bandU[i] = rng.next()
            bandV[i] = rng.next() * 2f - 1f
            bandRadius[i] = 0.07f + 0.09f * rng.next()
            bandAlpha[i] = 0.02f + 0.02f * rng.next()
            bandTint[i] = if (rng.next() < 0.75f) 0 else 1
        }
    }
}

private val STARS = StarField(STAR_COUNT)

/** What the engine said this minute. */
private class SkyNow(val chart: ChartRecord, val dayFactor: Float, val description: String)

/** A planet placed on the ring, with its glyph laid out. */
private class BodyMark(
    val x: Float,
    val y: Float,
    val above: Boolean,
    val color: Color,
    val radius: Float,
    val glyph: TextLayoutResult,
    val glyphX: Float,
    val glyphY: Float,
)

private fun ExactInstant.secs(): Double = taiSeconds.toDouble() + nanos.toDouble() / 1e9

/** 0 at −40 min, 1 at +40 min, linear between. */
private fun twilightRamp(x: Double): Float = ((x + TWILIGHT_SECONDS) / (2.0 * TWILIGHT_SECONDS)).coerceIn(0.0, 1.0).toFloat()

/**
 * How much daylight to tint the sky with: 1 between sunrise and sunset, 0 at night, a linear
 * blend across 40 minutes either side. `today` is the solar day containing now (its sunrise
 * is before now); `tomorrow` is the next solar day, whose sunrise gives the dawn twilight.
 */
private fun dayFactor(now: ExactInstant, today: PanchangaDetail?, tomorrow: PanchangaDetail?): Float {
    val t = now.secs()
    var f = 0f
    for (d in arrayOf(today, tomorrow)) {
        val rise = d?.sunrise ?: continue
        val set = d.sunset ?: continue
        f = max(f, minOf(twilightRamp(t - rise.secs()), twilightRamp(set.secs() - t)))
    }
    return f
}

private fun describe(place: Place, chart: ChartRecord): String {
    val sun = chart.bodies.firstOrNull { it.name == "Sun" }
    val moon = chart.bodies.firstOrNull { it.name == "Moon" }
    if (sun == null || moon == null) return "The sky over ${place.name} now"
    val elongation = norm360(moon.tropical - sun.tropical)
    return "The sky over ${place.name} now: Sun in ${signName(sun.tropical)}, Moon in ${signName(moon.tropical)}, ${phaseName(elongation)}"
}

/**
 * The Moon with its phase: a lit disc, the dark half, and the terminator as an ellipse whose
 * horizontal semi-axis is r·|cos e|. For a crescent (k < ½) the ellipse is dark and eats into
 * the lit half; for a gibbous Moon (k > ½) it is lit and spills into the dark half.
 * `litRight` says which limb faces the Sun.
 */
private fun DrawScope.drawMoonPhase(cx: Float, cy: Float, r: Float, elongationDeg: Double, litRight: Boolean, rim: Stroke) {
    val cosE = cos(Math.toRadians(norm360(elongationDeg)))
    val illuminated = ((1.0 - cosE) / 2.0).toFloat()
    val centre = Offset(cx, cy)
    drawCircle(KshanaColors.Text, r, centre)
    // The half away from the Sun: the left half runs clockwise from 6 o'clock, the right from 12.
    drawArc(KshanaColors.Night, if (litRight) 90f else -90f, 180f, true, Offset(cx - r, cy - r), Size(2f * r, 2f * r))
    val a = r * abs(cosE).toFloat()
    if (a > 0.01f) {
        val terminator = if (illuminated < 0.5f) KshanaColors.Night else KshanaColors.Text
        drawOval(terminator, Offset(cx - a, cy - r), Size(2f * a, 2f * r))
    }
    // Earthshine: the dark limb is just visible against the night.
    drawCircle(KshanaColors.TextSoft, r, centre, alpha = 0.18f, style = rim)
}

/**
 * The living sky over [place]: a seeded star field that twinkles and drifts, a faint Milky
 * Way, the ecliptic ring with its sign ticks, the Sun, the phase-correct Moon and the
 * naked-eye planets at their current longitudes (refreshed every minute from the engine),
 * the Ascendant marked on the horizon, and a day/night tint from sunrise and sunset.
 * [intensity] scales the star count and alpha (0.4 for screens that put text on top);
 * [content] renders over the sky.
 */
@Composable
fun LivingSky(
    place: Place,
    settings: EngineSettings,
    modifier: Modifier = Modifier,
    intensity: Float = 1f,
    content: @Composable BoxScope.() -> Unit = {},
) {
    val context = LocalContext.current
    var sky by remember { mutableStateOf<SkyNow?>(null) }

    // Positions once a minute, off the main thread. A failure keeps the last sky.
    LaunchedEffect(place, settings) {
        while (true) {
            try {
                val engine = EngineProvider.get(context)
                sky = withContext(Dispatchers.Default) {
                    val now = Instants.now()
                    val chart = engine.chart(now, place.latitude, place.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
                    val today = runCatching { engine.panchangaDetail(now, place.latitude, place.longitude, settings.ayanamsa) }.getOrNull()
                    val tomorrow = runCatching {
                        engine.panchangaDetail(ExactInstant(now.taiSeconds + 86_400L, now.nanos), place.latitude, place.longitude, settings.ayanamsa)
                    }.getOrNull()
                    SkyNow(chart, dayFactor(now, today, tomorrow), describe(place, chart))
                }
            } catch (e: Exception) {
                // The stars still shine without a chart.
            }
            delay(REFRESH_MILLIS)
        }
    }

    // Twinkle: one float 0..1 over 6 s, eased, restarting (sin is continuous across the wrap).
    val transition = rememberInfiniteTransition(label = "sky")
    val twinkle by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(6000, easing = EaseInOut), RepeatMode.Restart),
        label = "twinkle",
    )
    // Drift: the whole field slides 0.6 px per minute from the frame clock, so it breathes.
    var driftPx by remember { mutableFloatStateOf(0f) }
    LaunchedEffect(Unit) {
        val start = withFrameNanos { it }
        while (true) {
            withFrameNanos { now -> driftPx = ((now - start) * (DRIFT_PX_PER_MINUTE / 60e9)).toFloat() }
        }
    }

    val textMeasurer = rememberTextMeasurer()
    val labelStyle = remember { TextStyle(fontFamily = FontFamily.Monospace, fontSize = 9.sp) }
    val starAlpha = intensity.coerceIn(0f, 1f)
    val starCount = (STAR_COUNT * starAlpha).roundToInt().coerceIn(0, STAR_COUNT)
    val southern = place.latitude < 0.0
    val description = sky?.description ?: "The sky over ${place.name} now"

    Box(modifier.fillMaxSize().background(KshanaColors.Night)) {
        Canvas(
            modifier = Modifier
                .matchParentSize()
                .clipToBounds()
                .semantics { contentDescription = description }
                .drawWithCache {
                    val w = size.width
                    val h = size.height
                    if (w <= 0f || h <= 0f) return@drawWithCache onDrawBehind { }
                    val now = sky // state read here: the cache is rebuilt when the minute's chart arrives
                    val chart = now?.chart
                    val hasChart = chart != null
                    val dayFactor = now?.dayFactor ?: 0f

                    // ---- layout: horizon, and the ecliptic ellipse centred on it ----
                    val horizonY = h * HORIZON_FRACTION
                    val cx = w * 0.5f
                    val cy = horizonY
                    val rx = w * 0.46f
                    val ry = h * 0.22f
                    val dir = if (southern) -1f else 1f
                    val hair = 1.dp.toPx()

                    // ---- sky and ground ----
                    val topColor = lerp(KshanaColors.Night, DAY_BLUE, 0.35f * dayFactor)
                    val skyBrush = Brush.verticalGradient(0f to topColor, 0.7f to KshanaColors.Night, startY = 0f, endY = h)
                    val groundBrush = Brush.verticalGradient(
                        0f to KshanaColors.LineSoft.copy(alpha = 0.88f), 1f to KshanaColors.Night.copy(alpha = 0.96f),
                        startY = horizonY, endY = h,
                    )
                    val groundTopLeft = Offset(0f, horizonY)
                    val groundSize = Size(w, h - horizonY)
                    val horizonStart = Offset(0f, horizonY)
                    val horizonEnd = Offset(w, horizonY)

                    // ---- stars, in pixels, above the horizon; they fade in daylight ----
                    val n = starCount
                    val starX = FloatArray(n) { STARS.x[it] * w }
                    val starY = FloatArray(n) { STARS.y[it] * horizonY }
                    val starFade = starAlpha * (1f - 0.6f * dayFactor)

                    // ---- the Milky Way: soft blobs along a diagonal from upper left to right ----
                    val bandN = if (starAlpha > 0f) MILKY_WAY_BLOBS else 0
                    val bandX = FloatArray(bandN)
                    val bandY = FloatArray(bandN)
                    val bandR = FloatArray(bandN)
                    for (i in 0 until bandN) {
                        val u = STARS.bandU[i]
                        bandX[i] = w * (0.05f + 0.90f * u)
                        bandY[i] = horizonY * (0.08f + 0.62f * u) + STARS.bandV[i] * h * 0.07f
                        bandR[i] = STARS.bandRadius[i] * w
                    }
                    val bandBrush = Array(bandN) { i ->
                        val c = if (STARS.bandTint[i] == 0) KshanaColors.Text else KshanaColors.Sky
                        Brush.radialGradient(
                            0f to c.copy(alpha = STARS.bandAlpha[i] * starFade), 1f to c.copy(alpha = 0f),
                            center = Offset(bandX[i], bandY[i]), radius = bandR[i],
                        )
                    }

                    // ---- the ecliptic ring ----
                    val ringStroke = Stroke(hair)
                    val ringTopLeft = Offset(cx - rx, cy - ry)
                    val ringSize = Size(2f * rx, 2f * ry)
                    val asc = chart?.cuspsTropical?.firstOrNull() ?: 0.0
                    fun theta(lon: Double): Float = (PI + Math.toRadians(norm360(asc - lon))).toFloat()
                    fun above(lon: Double): Boolean = norm360(asc - lon) <= 180.0
                    fun ringX(th: Float): Float = cx + dir * rx * cos(th)
                    fun ringY(th: Float): Float = cy + ry * sin(th)
                    fun normalX(th: Float): Float {
                        val nx = dir * cos(th) / rx
                        val ny = sin(th) / ry
                        return nx / sqrt(nx * nx + ny * ny)
                    }
                    fun normalY(th: Float): Float {
                        val nx = dir * cos(th) / rx
                        val ny = sin(th) / ry
                        return ny / sqrt(nx * nx + ny * ny)
                    }

                    // Sign ticks across the boundaries, glyphs at the middles, outside the ring.
                    val tickX1 = FloatArray(12)
                    val tickY1 = FloatArray(12)
                    val tickX2 = FloatArray(12)
                    val tickY2 = FloatArray(12)
                    val glyphX = FloatArray(12)
                    val glyphY = FloatArray(12)
                    val glyphs = Array(12) { textMeasurer.measure(SIGN_GLYPHS[it], labelStyle) }
                    val tickIn = 3.dp.toPx()
                    val tickOut = 5.dp.toPx()
                    val glyphOut = 14.dp.toPx()
                    for (i in 0 until 12) {
                        val th = theta(i * 30.0)
                        val px = ringX(th)
                        val py = ringY(th)
                        val nx = normalX(th)
                        val ny = normalY(th)
                        tickX1[i] = px - nx * tickIn
                        tickY1[i] = py - ny * tickIn
                        tickX2[i] = px + nx * tickOut
                        tickY2[i] = py + ny * tickOut
                        val thm = theta(i * 30.0 + 15.0)
                        glyphX[i] = ringX(thm) + normalX(thm) * glyphOut - glyphs[i].size.width / 2f
                        glyphY[i] = ringY(thm) + normalY(thm) * glyphOut - glyphs[i].size.height / 2f
                    }

                    // ---- the planets ----
                    val labelGap = 3.dp.toPx()
                    val bodies: Array<BodyMark> = if (chart == null) emptyArray() else chart.bodies
                        .filter { it.name in VISIBLE_PLANETS }
                        .map { b ->
                            val th = theta(b.tropical)
                            val x = ringX(th)
                            val y = ringY(th)
                            val color = when (b.name) {
                                "Venus", "Jupiter" -> KshanaColors.Sky
                                else -> KshanaColors.Text
                            }
                            val radiusDp = when (b.name) {
                                "Venus" -> 2.4f
                                "Jupiter" -> 2.2f
                                "Mars" -> 1.9f
                                "Saturn" -> 1.7f
                                else -> 1.5f
                            }
                            val radius = radiusDp.dp.toPx()
                            val glyph = textMeasurer.measure(PLANET_GLYPHS[b.name] ?: b.name.take(2), labelStyle)
                            BodyMark(x, y, above(b.tropical), color, radius, glyph, x + radius + labelGap, y - glyph.size.height / 2f)
                        }
                        .toTypedArray()

                    // ---- the Sun: a soft gold disc with a halo; under the ground only a dim ring ----
                    val sun = chart?.bodies?.firstOrNull { it.name == "Sun" }
                    val moon = chart?.bodies?.firstOrNull { it.name == "Moon" }
                    val sunTh = theta(sun?.tropical ?: 0.0)
                    val sunX = ringX(sunTh)
                    val sunY = ringY(sunTh)
                    val sunAbove = sun != null && above(sun.tropical)
                    val sunR = 7.dp.toPx()
                    val haloR = 32.dp.toPx()
                    val markR = 4.dp.toPx()
                    val markStroke = Stroke(hair)
                    val haloScale = if (sunAbove) 1f else 0.3f
                    val sunHalo = Brush.radialGradient(
                        0f to KshanaColors.Gold.copy(alpha = 0.55f * haloScale),
                        0.3f to KshanaColors.Gold.copy(alpha = 0.14f * haloScale),
                        1f to KshanaColors.Gold.copy(alpha = 0f),
                        center = Offset(sunX, sunY), radius = haloR,
                    )
                    val sunGlyph = textMeasurer.measure("☉", labelStyle)
                    val sunGlyphX = sunX + (if (sunAbove) sunR else markR) + labelGap
                    val sunGlyphY = sunY - sunGlyph.size.height / 2f

                    // ---- the Moon: phase from the Moon − Sun elongation; the lit limb faces the Sun ----
                    val moonTh = theta(moon?.tropical ?: 0.0)
                    val moonX = ringX(moonTh)
                    val moonY = ringY(moonTh)
                    val moonAbove = moon != null && above(moon.tropical)
                    val elongation = if (sun != null && moon != null) norm360(moon.tropical - sun.tropical) else 0.0
                    // Waxing: the Moon trails the Sun, so its lit limb faces west, the right side north of the equator.
                    val litRight = (elongation < 180.0) != southern
                    val moonR = 6.dp.toPx()
                    val moonMarkR = 3.5.dp.toPx()
                    val rimStroke = Stroke(max(1f, 0.5.dp.toPx()))
                    val moonGlyph = textMeasurer.measure("☽", labelStyle)
                    val moonGlyphX = moonX + (if (moonAbove) moonR else moonMarkR) + labelGap
                    val moonGlyphY = moonY - moonGlyph.size.height / 2f

                    // ---- the Ascendant: a gold tick on the horizon at the east end of the ring ----
                    val ascTh = theta(asc)
                    val ascX = ringX(ascTh)
                    val ascY = ringY(ascTh)
                    val ascLabel = textMeasurer.measure(
                        "↑ Asc ${(norm360(asc) % 30.0).toInt()}° ${SIGN_GLYPHS[signIndex(asc)]}", labelStyle,
                    )
                    val ascGap = 6.dp.toPx()
                    val ascLabelX = if (southern) ascX - ascGap - ascLabel.size.width else ascX + ascGap
                    val ascLabelY = ascY - 10.dp.toPx() - ascLabel.size.height
                    val ascTickTop = Offset(ascX, ascY - 8.dp.toPx())
                    val ascTickBottom = Offset(ascX, ascY + 4.dp.toPx())
                    val ascTickWidth = 1.5.dp.toPx()

                    onDrawBehind {
                        // The only state read per frame: the two clocks.
                        val phaseT = twinkle * TWO_PI
                        val drift = driftPx

                        drawRect(skyBrush)
                        for (i in 0 until bandN) drawCircle(bandBrush[i], bandR[i], Offset(bandX[i], bandY[i]))
                        for (i in 0 until n) {
                            val x = (starX[i] + drift) % w
                            val a = (0.55f + 0.45f * sin(STARS.phase[i] + phaseT)) * STARS.brightness[i] * starFade
                            val c = when (STARS.tint[i]) {
                                1 -> KshanaColors.Sky
                                2 -> KshanaColors.Gold
                                else -> KshanaColors.Text
                            }
                            drawCircle(c, STARS.radius[i], Offset(x, starY[i]), alpha = a)
                        }

                        // The ecliptic, its ticks and glyphs; the lower half is then veiled by the ground.
                        drawOval(KshanaColors.Gold, ringTopLeft, ringSize, alpha = 0.35f, style = ringStroke)
                        if (hasChart) {
                            for (i in 0 until 12) {
                                drawLine(KshanaColors.Gold, Offset(tickX1[i], tickY1[i]), Offset(tickX2[i], tickY2[i]), strokeWidth = hair, alpha = 0.5f)
                                drawText(glyphs[i], color = KshanaColors.Muted, topLeft = Offset(glyphX[i], glyphY[i]), alpha = 0.85f)
                            }
                        }
                        drawRect(groundBrush, groundTopLeft, groundSize)
                        drawLine(KshanaColors.Line, horizonStart, horizonEnd, strokeWidth = hair, alpha = 0.9f)

                        if (hasChart) {
                            for (b in bodies) {
                                val a = if (b.above) 1f else 0.3f
                                drawCircle(b.color, b.radius, Offset(b.x, b.y), alpha = a)
                                drawText(b.glyph, color = b.color, topLeft = Offset(b.glyphX, b.glyphY), alpha = a * 0.85f)
                            }
                            drawCircle(sunHalo, haloR, Offset(sunX, sunY))
                            if (sunAbove) {
                                drawCircle(KshanaColors.Gold, sunR, Offset(sunX, sunY))
                            } else {
                                drawCircle(KshanaColors.Gold, markR, Offset(sunX, sunY), alpha = 0.4f, style = markStroke)
                            }
                            drawText(sunGlyph, color = KshanaColors.Gold, topLeft = Offset(sunGlyphX, sunGlyphY), alpha = if (sunAbove) 0.9f else 0.4f)
                            if (moonAbove) {
                                drawMoonPhase(moonX, moonY, moonR, elongation, litRight, rimStroke)
                            } else {
                                drawCircle(KshanaColors.TextSoft, moonMarkR, Offset(moonX, moonY), alpha = 0.35f, style = markStroke)
                            }
                            drawText(moonGlyph, color = KshanaColors.TextSoft, topLeft = Offset(moonGlyphX, moonGlyphY), alpha = if (moonAbove) 0.9f else 0.35f)
                            drawLine(KshanaColors.Gold, ascTickTop, ascTickBottom, strokeWidth = ascTickWidth)
                            drawText(ascLabel, color = KshanaColors.Gold, topLeft = Offset(ascLabelX, ascLabelY))
                        }
                    }
                },
        ) { }
        content()
    }
}

/**
 * The phase-correct Moon alone, [size] across, lit limb on the right while waxing (the
 * northern-hemisphere view). [elongationDeg] is Moon − Sun longitude: 0 new, 90 first
 * quarter, 180 full, 270 last quarter.
 */
@Composable
fun MoonDisc(elongationDeg: Double, size: Dp, modifier: Modifier = Modifier) {
    val rim = remember { Stroke(1f) }
    val litRight = norm360(elongationDeg) < 180.0
    val label = "Moon, ${phaseName(elongationDeg)}"
    Canvas(modifier.size(size).semantics { contentDescription = label }) {
        val r = this.size.minDimension / 2f - 1f
        if (r > 0f) drawMoonPhase(this.size.width / 2f, this.size.height / 2f, r, elongationDeg, litRight, rim)
    }
}

/**
 * A ring whose arc shows [progress] in [0, 1] from 12 o'clock and whose stroke alpha
 * breathes on a 4 s cycle, over a faint full track. Size it with [modifier] (44 dp if none).
 */
@Composable
fun BreathingRing(progress: Float, modifier: Modifier = Modifier, color: Color = KshanaColors.Gold) {
    val transition = rememberInfiniteTransition(label = "ring")
    val breath by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(2000, easing = EaseInOut), RepeatMode.Reverse),
        label = "breath",
    )
    val density = LocalDensity.current
    val stroke = remember(density) { Stroke(width = with(density) { 2.dp.toPx() }, cap = StrokeCap.Round) }
    val clamped = progress.coerceIn(0f, 1f)
    Canvas(
        modifier
            .defaultMinSize(44.dp, 44.dp)
            .semantics { progressBarRangeInfo = ProgressBarRangeInfo(clamped, 0f..1f) },
    ) {
        val d = size.minDimension - stroke.width
        if (d <= 0f) return@Canvas
        val topLeft = Offset((size.width - d) / 2f, (size.height - d) / 2f)
        val arcSize = Size(d, d)
        drawArc(color, 0f, 360f, false, topLeft, arcSize, alpha = 0.15f, style = stroke)
        val sweep = 360f * clamped
        if (sweep > 0f) drawArc(color, -90f, sweep, false, topLeft, arcSize, alpha = 0.45f + 0.55f * breath, style = stroke)
    }
}
