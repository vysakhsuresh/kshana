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
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
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
import androidx.compose.ui.graphics.Paint
import androidx.compose.ui.graphics.PointMode
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.translate
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
import uniffi.kshana_ffi.Engine
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.sin
import kotlin.math.sqrt

/*
 * The living sky: a full-bleed night canvas drawn, offline, from the real sky over a place.
 *
 * Stars. The Yale Bright Star Catalogue to V 5.0 (assets/sky/bsc.bin, see Stars.kt),
 * precessed to the date and placed from the engine's local sidereal time and the place
 * latitude, so the sky over the phone is the sky over the person: Orion rises in the east
 * when Orion rises. The dome is projected stereographically from the horizon point the
 * observer faces (south north of the equator, north south of it): the horizon is the line
 * at 80 % of the height, the east and west points are the ends of the ecliptic ring, the
 * zenith is one ring-half-width above the horizon and the sky beyond it wraps on towards
 * the top edge. Positions are rewritten in place every 20 s (the sky turns 0.25°/min) and
 * the chart is refreshed once a minute.
 *
 * Light. The Sun's altitude (from the chart: Sun longitude, obliquity, sidereal time)
 * drives the tint, the horizon band and the limiting magnitude: day above +6°, golden hour
 * to 0°, civil twilight to −6° (stars brighter than 1.5), nautical to −12° (3.5),
 * astronomical to −18° (5.0), night below. A Moon above the horizon thins the field by up
 * to 30 % of its illuminated fraction and warms the sky 2 %.
 *
 * Motion. Everything animated reads the motion contract (Motion.kt): twinkle, the 20 s
 * sidereal drift and the ≤ 8 dp tilt parallax stop under it and the sky is one static
 * frame refreshed once a minute. All geometry lives in float arrays, brushes and text
 * layouts built in `drawWithCache` (when the size or the minute's chart changes); the
 * per-frame `onDrawBehind` reads two clocks and the tilt and allocates nothing: the faint
 * stars go out as twelve `drawRawPoints` batches, the bright ones as discs.
 *
 * The ecliptic ring, bodies and Ascendant are the cartoon they were: the ring is an
 * ellipse on the horizon, a body's place along it is its longitude distance back from the
 * Ascendant (0° east, 90° top, 180° west), everything tropical like the wheel on Now.
 */

private const val TWO_PI = 6.2831855f
private const val HORIZON_FRACTION = 0.80f
private const val REFRESH_MILLIS = 60_000L
private const val PROJECT_NANOS = 20_000_000_000L
private const val TWINKLE_NANOS = Motion.TWINKLE_PERIOD_MS * 1_000_000L
private val DAY_BLUE = Color(0xFF0E1A3A)
private val DAY_HAZE = Color(0xFF6E8FC4)
private val DUSK_ORANGE = Color(0xFFD9733A)
private val DUSK_VIOLET = Color(0xFF3A2F6E)
private val MOON_WARM = Color(0xFF2A2820)

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

/** What the engine said this minute, with the light derived from it. */
private class SkyNow(
    val chart: ChartRecord,
    /** System.nanoTime() when the chart was computed; the sidereal time advances from here. */
    val atNanos: Long,
    val sunAltDeg: Double,
    val moonAltDeg: Double,
    /** Illuminated fraction of the Moon, 0 new to 1 full. */
    val moonIlluminated: Double,
    /** The J2000 → date precession matrix for the chart's TT date. */
    val precession: DoubleArray,
    val description: String,
) {
    /** The local sidereal time at frame time [nowNanos], degrees. */
    fun lstAt(nowNanos: Long): Double = chart.lstDeg + Stars.LST_DEG_PER_SECOND * ((nowNanos - atNanos) / 1e9)
}

/** The sky light for a solar and lunar altitude. */
private class SkyLight(
    /** The faintest magnitude the sky lets through. */
    val limitingMag: Float,
    /** 0 at night (−6° and below), 1 by day (+6° and above). */
    val dayFactor: Float,
    val top: Color,
    val bandColor: Color,
    val bandAlpha: Float,
    /** Star alpha factor for moonlight, 1 with no Moon. */
    val moonDim: Float,
    val bandName: String,
)

private val LIMIT_H = doubleArrayOf(6.0, 0.0, -6.0, -12.0, -18.0, -24.0)
private val LIMIT_M = floatArrayOf(-3f, -2f, 1.5f, 3.5f, 5.0f, 6.5f)

/** Limiting magnitude by solar altitude, linear between the twilight rows. */
private fun limitingMagnitude(h: Double): Float {
    if (h >= LIMIT_H[0]) return LIMIT_M[0]
    for (i in 1 until LIMIT_H.size) {
        if (h >= LIMIT_H[i]) {
            val t = ((LIMIT_H[i - 1] - h) / (LIMIT_H[i - 1] - LIMIT_H[i])).toFloat()
            return LIMIT_M[i - 1] + (LIMIT_M[i] - LIMIT_M[i - 1]) * t
        }
    }
    return LIMIT_M.last()
}

private val BAND_H = doubleArrayOf(8.0, 6.0, 0.0, -6.0, -12.0, -18.0)
private val BAND_C = arrayOf(DAY_HAZE, KshanaColors.Gold, DUSK_ORANGE, DUSK_VIOLET, DUSK_VIOLET, DUSK_VIOLET)
private val BAND_A = floatArrayOf(0.16f, 0.22f, 0.28f, 0.16f, 0.05f, 0f)

private fun skyLight(now: SkyNow?): SkyLight {
    val h = now?.sunAltDeg ?: -30.0
    val dayFactor = ((h + 6.0) / 12.0).coerceIn(0.0, 1.0).toFloat()
    val moonUp = now != null && now.moonAltDeg > 0.0
    val k = if (moonUp) now!!.moonIlluminated.toFloat() * ((now.moonAltDeg / 20.0).coerceIn(0.0, 1.0)).toFloat() else 0f
    var top = lerp(KshanaColors.Night, DAY_BLUE, 0.35f * dayFactor)
    top = lerp(top, MOON_WARM, 0.02f * k)
    var bandColor = DUSK_VIOLET
    var bandAlpha = 0f
    if (h >= BAND_H[0]) {
        bandColor = BAND_C[0]; bandAlpha = BAND_A[0]
    } else {
        for (i in 1 until BAND_H.size) {
            if (h >= BAND_H[i]) {
                val t = ((BAND_H[i - 1] - h) / (BAND_H[i - 1] - BAND_H[i])).toFloat()
                bandColor = lerp(BAND_C[i - 1], BAND_C[i], t)
                bandAlpha = BAND_A[i - 1] + (BAND_A[i] - BAND_A[i - 1]) * t
                break
            }
        }
    }
    val name = when {
        h >= 0.0 -> "day"
        h >= -6.0 -> "civil twilight"
        h >= -12.0 -> "nautical twilight"
        h >= -18.0 -> "astronomical twilight"
        else -> "night"
    }
    return SkyLight(limitingMagnitude(h), dayFactor, top, bandColor, bandAlpha, 1f - 0.3f * k, name)
}

/** The canvas size the cache last saw, for the frame loop's re-projection. */
private class SkyGeometry {
    var width = 0f
    var height = 0f
}

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

private fun describe(place: Place, chart: ChartRecord, light: String): String {
    val sun = chart.bodies.firstOrNull { it.name == "Sun" }
    val moon = chart.bodies.firstOrNull { it.name == "Moon" }
    if (sun == null || moon == null) return "The sky over ${place.name} now, $light"
    val elongation = norm360(moon.tropical - sun.tropical)
    return "The sky over ${place.name} now: Sun in ${signName(sun.tropical)}, Moon in ${signName(moon.tropical)}, ${phaseName(elongation)}; $light"
}

/** One engine call per minute: the chart, and from it the Sun's and Moon's altitude. */
private fun computeSky(engine: Engine, place: Place, settings: EngineSettings): SkyNow {
    val now = Instants.now()
    val chart = engine.chart(now, place.latitude, place.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
    val sun = chart.bodies.firstOrNull { it.name == "Sun" }
    val moon = chart.bodies.firstOrNull { it.name == "Moon" }
    val eq = DoubleArray(2)
    // The Sun's altitude from the ephemeris; if that call fails, the geometric altitude from
    // the chart's own Sun longitude, obliquity and sidereal time (good to about 1′).
    val sunAlt = runCatching { engine.sunAltitude(now, place.latitude, place.longitude) }.getOrElse {
        if (sun != null) {
            Stars.eclipticToEquatorial(sun.tropical, sun.latitude, chart.obliquityDeg, eq)
            Stars.altitudeDeg(eq[0], eq[1], chart.lstDeg, place.latitude)
        } else -30.0
    }
    val moonAlt = if (moon != null) {
        Stars.eclipticToEquatorial(moon.tropical, moon.latitude, chart.obliquityDeg, eq)
        Stars.altitudeDeg(eq[0], eq[1], chart.lstDeg, place.latitude)
    } else -30.0
    val illuminated = if (sun != null && moon != null) (1.0 - cos(Math.toRadians(norm360(moon.tropical - sun.tropical)))) / 2.0 else 0.0
    val precession = DoubleArray(9)
    val t = ((chart.ttJd1 - 2451545.0) + chart.ttJd2) / 36525.0
    if (abs(t) < 10.0) Stars.precessionMatrix(t, precession) else Stars.identity(precession)
    val lightName = when {
        sunAlt >= 0.0 -> "day"
        sunAlt >= -6.0 -> "civil twilight"
        sunAlt >= -12.0 -> "nautical twilight"
        sunAlt >= -18.0 -> "astronomical twilight"
        else -> "night"
    }
    return SkyNow(chart, System.nanoTime(), sunAlt, moonAlt, illuminated, precession, describe(place, chart, lightName))
}

/** Rewrite the star arrays for the sky at frame time [nowNanos] and the canvas [w] × [h]. */
private fun projectStars(proj: StarProjection, sky: SkyNow, nowNanos: Long, w: Float, h: Float, latitude: Double, southern: Boolean, intensity: Float) {
    val light = skyLight(sky)
    proj.project(
        precession = sky.precession,
        lstDeg = sky.lstAt(nowNanos),
        latDeg = latitude,
        southern = southern,
        cx = w * 0.5f,
        horizonY = h * HORIZON_FRACTION,
        scale = w * 0.46f,
        width = w,
        height = h,
        limitingMag = light.limitingMag,
        moonDim = light.moonDim,
        intensity = intensity,
    )
}

private fun starColor(tint: Int): Color = when (tint) {
    0 -> lerp(KshanaColors.Text, KshanaColors.Gold, 0.5f)
    2 -> lerp(KshanaColors.Text, KshanaColors.Sky, 0.7f)
    else -> KshanaColors.Text
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
 * The living sky over [place]: the real stars of the Bright Star Catalogue placed for the
 * observer and the minute, lit by the Sun's altitude (twilight bands, limiting magnitude)
 * and thinned by the Moon; the ecliptic ring with its sign ticks, the Sun, the phase-correct
 * Moon and the naked-eye planets at their current longitudes (refreshed every minute from
 * the engine), and the Ascendant marked on the horizon. Twinkle, sidereal drift and tilt
 * parallax obey the motion contract. [intensity] scales the stars' alpha and, at ≥ 0.9,
 * shows the names of the first-magnitude stars (0.4 for screens that put text on top);
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
    KshanaFonts.attach(context)
    remember(context) { Foreground.register(context); 0 }
    val reduced = Motion.rememberReduced()
    val starAlpha = intensity.coerceIn(0f, 1f)
    val southern = place.latitude < 0.0

    val catalogue by produceState<StarCatalogue?>(initialValue = null, context) {
        value = withContext(Dispatchers.IO) { Stars.load(context) }
    }
    val projection = remember(catalogue) { catalogue?.let { StarProjection(it) } }
    var sky by remember { mutableStateOf<SkyNow?>(null) }
    var projTick by remember { mutableIntStateOf(0) }
    var twinkleT by remember { mutableFloatStateOf(0f) }
    val geometry = remember { SkyGeometry() }
    val tilt = rememberTilt(enabled = !reduced && starAlpha > 0f)

    // The chart once a minute, off the main thread. A failure keeps the last sky.
    LaunchedEffect(place, settings) {
        while (true) {
            try {
                val engine = EngineProvider.get(context)
                val next = withContext(Dispatchers.Default) { computeSky(engine, place, settings) }
                sky = next
                SkySound.setSky(next.sunAltDeg)
                SkySound.resume(context)
            } catch (e: Exception) {
                // The stars still shine without a chart.
            }
            delay(REFRESH_MILLIS)
        }
    }

    // The frame loop: twinkle clock, the 20 s re-projection (the real sidereal drift), and
    // the foreground signal. Under the contract it ticks once a second and moves nothing.
    LaunchedEffect(reduced, projection) {
        val origin = withFrameNanos { it }
        var lastProjected = Long.MIN_VALUE
        var lastSoundCheck = Long.MIN_VALUE
        var lastSky: SkyNow? = null
        while (true) {
            val now = withFrameNanos { it }
            Foreground.sawFrame()
            if (!reduced) twinkleT = ((now - origin) % TWINKLE_NANOS) / TWINKLE_NANOS.toFloat()
            if (now - lastSoundCheck >= 5_000_000_000L) {
                // Frames flowing again after the UI was hidden: the drone may come back.
                lastSoundCheck = now
                SkySound.resume(context)
            }
            val s = sky
            val w = geometry.width
            val h = geometry.height
            if (projection != null && s != null && w > 0f && h > 0f) {
                val due = s !== lastSky || now - lastProjected >= PROJECT_NANOS || w != projection.lastWidth || h != projection.lastHeight
                if (due) {
                    projectStars(projection, s, now, w, h, place.latitude, southern, starAlpha)
                    projTick++
                    lastProjected = now
                    lastSky = s
                }
            }
            if (reduced) delay(1000L)
        }
    }

    val textMeasurer = rememberTextMeasurer()
    val fontsBundled = KshanaFonts.bundled
    val labelStyle = remember(fontsBundled) { TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp) }
    val description = sky?.description ?: "The sky over ${place.name} now"
    val density = LocalDensity.current

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
                    val light = skyLight(now)
                    val proj = projection
                    geometry.width = w
                    geometry.height = h
                    if (proj != null && now != null && (proj.lastWidth != w || proj.lastHeight != h)) {
                        projectStars(proj, now, System.nanoTime(), w, h, place.latitude, southern, starAlpha)
                    }

                    // ---- layout: horizon, and the ecliptic ellipse centred on it ----
                    val horizonY = h * HORIZON_FRACTION
                    val cx = w * 0.5f
                    val cy = horizonY
                    val rx = w * 0.46f
                    val ry = h * 0.22f
                    val dir = if (southern) -1f else 1f
                    val hair = 1.dp.toPx()
                    val px = density.density
                    val ringParallax = Motion.PARALLAX_RING_DP / Motion.PARALLAX_STARS_DP

                    // ---- sky, twilight band and ground ----
                    val skyBrush = Brush.verticalGradient(0f to light.top, 0.7f to KshanaColors.Night, startY = 0f, endY = h)
                    val bandTop = horizonY * 0.72f
                    val bandBrush = Brush.verticalGradient(
                        0f to light.bandColor.copy(alpha = 0f), 1f to light.bandColor.copy(alpha = light.bandAlpha),
                        startY = bandTop, endY = horizonY,
                    )
                    val bandTopLeft = Offset(0f, bandTop)
                    val bandSize = Size(w, horizonY - bandTop)
                    val bandOn = light.bandAlpha > 0.002f
                    val groundBrush = Brush.verticalGradient(
                        0f to KshanaColors.LineSoft.copy(alpha = 0.88f), 1f to KshanaColors.Night.copy(alpha = 0.96f),
                        startY = horizonY, endY = h,
                    )
                    val groundTopLeft = Offset(0f, horizonY)
                    val groundSize = Size(w, h - horizonY)
                    val horizonStart = Offset(0f, horizonY)
                    val horizonEnd = Offset(w, horizonY)

                    // ---- stars: paints for the twelve point buckets, colours for the discs ----
                    val bucketPaints = Array(StarProjection.BUCKETS) { b ->
                        Paint().apply {
                            color = starColor(b % StarProjection.TINTS)
                            strokeWidth = 2f * StarProjection.radiusDp(StarProjection.bandMag(b / StarProjection.TINTS)) * px
                            strokeCap = StrokeCap.Round
                            isAntiAlias = true
                        }
                    }
                    val discColor = if (proj != null) Array(proj.nBright) { starColor(proj.cat.tint[it].toInt()) } else emptyArray()
                    val discRadius = if (proj != null) FloatArray(proj.nBright) { StarProjection.radiusDp(proj.cat.mag[it]) * px } else FloatArray(0)
                    val haloFrom = if (proj != null) {
                        var n = 0
                        while (n < proj.nBright && proj.cat.mag[n] < 1.5f) n++
                        n
                    } else 0
                    val drawStars = proj != null && starAlpha > 0f

                    // Names of the first-magnitude stars, only on a quiet sky.
                    val labelIdx: IntArray
                    val labels: Array<TextLayoutResult>
                    if (proj != null && starAlpha >= 0.9f) {
                        val idx = proj.cat.namedIndices.filter { it < proj.nBright && proj.cat.mag[it] < 1.5f }
                        labelIdx = idx.toIntArray()
                        labels = Array(idx.size) { textMeasurer.measure(proj.cat.name[idx[it]] ?: "", labelStyle) }
                    } else {
                        labelIdx = IntArray(0)
                        labels = emptyArray()
                    }
                    val labelGap = 3.dp.toPx()
                    val labelAlpha = 0.75f * (1f - light.dayFactor)

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
                        val pxr = ringX(th)
                        val pyr = ringY(th)
                        val nx = normalX(th)
                        val ny = normalY(th)
                        tickX1[i] = pxr - nx * tickIn
                        tickY1[i] = pyr - ny * tickIn
                        tickX2[i] = pxr + nx * tickOut
                        tickY2[i] = pyr + ny * tickOut
                        val thm = theta(i * 30.0 + 15.0)
                        glyphX[i] = ringX(thm) + normalX(thm) * glyphOut - glyphs[i].size.width / 2f
                        glyphY[i] = ringY(thm) + normalY(thm) * glyphOut - glyphs[i].size.height / 2f
                    }

                    // ---- the planets ----
                    val bodyGap = 3.dp.toPx()
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
                            BodyMark(x, y, above(b.tropical), color, radius, glyph, x + radius + bodyGap, y - glyph.size.height / 2f)
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
                    val sunGlyphX = sunX + (if (sunAbove) sunR else markR) + bodyGap
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
                    val moonGlyphX = moonX + (if (moonAbove) moonR else moonMarkR) + bodyGap
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
                        // The only state read per frame: the two clocks and the tilt.
                        val phaseT = twinkleT * TWO_PI
                        @Suppress("UNUSED_VARIABLE") val tick = projTick // subscribes this draw to the 20 s re-projection
                        val tx = tilt.x * px
                        val ty = tilt.y * px

                        drawRect(skyBrush)
                        if (bandOn) drawRect(bandBrush, bandTopLeft, bandSize)

                        if (drawStars && proj != null) {
                            translate(tx, ty) {
                                // Faint stars: twelve batches, a whisper of shimmer per batch.
                                for (b in 0 until StarProjection.BUCKETS) {
                                    val a = proj.bucketAlpha[b]
                                    if (a <= 0.004f) continue
                                    val paint = bucketPaints[b]
                                    paint.alpha = (a * (0.92f + 0.08f * sin(phaseT + b))).coerceIn(0f, 1f)
                                    drawContext.canvas.drawRawPoints(PointMode.Points, proj.pts[b], paint)
                                }
                                // Bright stars: discs with their own twinkle; the first magnitude gets a halo.
                                for (i in 0 until proj.nBright) {
                                    if (!proj.bVisible[i]) continue
                                    val amp = proj.bTwinkle[i]
                                    val a = (proj.bAlpha[i] * (1f - amp + amp * sin(proj.bPhase[i] + phaseT))).coerceIn(0f, 1f)
                                    if (a <= 0.004f) continue
                                    val c = discColor[i]
                                    val r = discRadius[i]
                                    if (i < haloFrom) drawCircle(c, r * 2.4f, Offset(proj.bx[i], proj.by[i]), alpha = a * 0.16f)
                                    drawCircle(c, r, Offset(proj.bx[i], proj.by[i]), alpha = a)
                                }
                                for (k in labelIdx.indices) {
                                    val i = labelIdx[k]
                                    if (!proj.bVisible[i]) continue
                                    val layout = labels[k]
                                    drawText(
                                        layout, color = KshanaColors.Muted,
                                        topLeft = Offset(proj.bx[i] + discRadius[i] + labelGap, proj.by[i] - layout.size.height / 2f),
                                        alpha = labelAlpha * proj.bAlpha[i],
                                    )
                                }
                            }
                        }

                        // The ecliptic, its ticks and glyphs; the lower half is then veiled by the ground.
                        translate(tx * ringParallax, ty * ringParallax) {
                            drawOval(KshanaColors.Gold, ringTopLeft, ringSize, alpha = 0.35f, style = ringStroke)
                            if (hasChart) {
                                for (i in 0 until 12) {
                                    drawLine(KshanaColors.Gold, Offset(tickX1[i], tickY1[i]), Offset(tickX2[i], tickY2[i]), strokeWidth = hair, alpha = 0.5f)
                                    drawText(glyphs[i], color = KshanaColors.Muted, topLeft = Offset(glyphX[i], glyphY[i]), alpha = 0.85f)
                                }
                            }
                        }
                        drawRect(groundBrush, groundTopLeft, groundSize)
                        drawLine(KshanaColors.Line, horizonStart, horizonEnd, strokeWidth = hair, alpha = 0.9f)

                        if (hasChart) {
                            translate(tx * ringParallax, ty * ringParallax) {
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
                            }
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
 * breathes on a 5 s cycle (0.2 Hz), over a faint full track; still under the motion
 * contract. Size it with [modifier] (44 dp if none).
 */
@Composable
fun BreathingRing(progress: Float, modifier: Modifier = Modifier, color: Color = KshanaColors.Gold) {
    val reduced = Motion.rememberReduced()
    val breathState: State<Float>? = if (reduced) null else {
        val transition = rememberInfiniteTransition(label = "ring")
        transition.animateFloat(
            initialValue = 0f,
            targetValue = 1f,
            animationSpec = infiniteRepeatable(tween(Motion.BREATH_PERIOD_MS / 2, easing = EaseInOut), RepeatMode.Reverse),
            label = "breath",
        )
    }
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
        val breath = breathState?.value ?: 0.55f // read in the draw phase: no recomposition per frame
        val topLeft = Offset((size.width - d) / 2f, (size.height - d) / 2f)
        val arcSize = Size(d, d)
        drawArc(color, 0f, 360f, false, topLeft, arcSize, alpha = 0.15f, style = stroke)
        val sweep = 360f * clamped
        if (sweep > 0f) drawArc(color, -90f, sweep, false, topLeft, arcSize, alpha = 0.45f + 0.55f * breath, style = stroke)
    }
}
