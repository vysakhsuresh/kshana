package com.kshana.app.ui.onboarding

import android.animation.ValueAnimator
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.FloatState
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.screens.BirthScreen
import com.kshana.app.ui.screens.GoldButton
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ChartRecord
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.sqrt
import com.kshana.app.ui.sky.KshanaFonts

/*
 * The first run as a ritual, in three steps and under a minute. (1) An invocation: a near-black
 * screen where one serif line types itself. (2) "When did you arrive?": the existing birth form,
 * which saves the profile. (3) The rewind: a full-screen sky over the birth place whose planets
 * slide from where they are now to where they were at the birth instant, while a monospace
 * clock counts backwards through years, days and seconds; it ends on the birth sky held still
 * and the door in. Under reduced motion (animators disabled by the system or battery saver)
 * nothing types and nothing slides: the words and the final sky are shown at once.
 */

private const val LINE_ONE = "Everything here is computed, nothing is guessed."
private const val LINE_TWO = "When did you arrive?"
private const val TYPE_MS = 42L
private const val REWIND_MILLIS = 6_000.0
private const val HOLD_BEFORE_MS = 900L
private const val STAR_COUNT = 320
private const val STAR_SEED = 0x415252495641L // "ARRIVA": the same stars on every arrival
private const val HORIZON_FRACTION = 0.80f
private val DAY_BLUE = Color(0xFF0E1A3A)
private val STAMP get() = com.kshana.app.ui.screens.datePattern("d MMM uuuu  HH:mm:ss")
private val SIGN_GLYPHS = listOf("♈", "♉", "♊", "♋", "♌", "♍", "♎", "♏", "♐", "♑", "♒", "♓")
private val BODY_GLYPHS = mapOf(
    "Sun" to "☉", "Moon" to "☽", "Mercury" to "☿", "Venus" to "♀", "Mars" to "♂", "Jupiter" to "♃", "Saturn" to "♄",
)
/** Drawn in this order so the Sun and Moon sit on top of the planets. */
private val DRAWN_BODIES = listOf("Mercury", "Venus", "Mars", "Jupiter", "Saturn", "Sun", "Moon")

private fun norm360(x: Double): Double = (x % 360.0 + 360.0) % 360.0

/** The signed distance from [from] to [to] the short way around, in (−180, 180]. */
private fun shortWay(from: Double, to: Double): Double {
    val d = norm360(to - from)
    return if (d > 180.0) d - 360.0 else d
}

/**
 * Slow at both ends, fast in the middle: t⁴ / (t⁴ + (1 − t)⁴). Over thirty years the first
 * second rewinds hours, the middle second whole years, and the last second settles through
 * days to the exact second.
 */
private fun ease(t: Float): Float {
    val x = t.coerceIn(0f, 1f)
    val a = x * x * x * x
    val y = 1f - x
    val b = y * y * y * y
    return if (a + b <= 0f) 0f else a / (a + b)
}

/** A seeded linear congruential generator (Knuth's MMIX constants). */
private class Lcg(private var state: Long) {
    fun next(): Float {
        state = state * 6364136223846793005L + 1442695040888963407L
        return ((state ushr 40).toInt() and 0xFFFFFF) / 16777216f
    }
}

private class Stars(count: Int) {
    val x = FloatArray(count)
    val y = FloatArray(count)
    val radius = FloatArray(count)
    val alpha = FloatArray(count)
    val tint = IntArray(count)

    init {
        val rng = Lcg(STAR_SEED)
        for (i in 0 until count) {
            x[i] = rng.next()
            y[i] = rng.next()
            val r = rng.next()
            radius[i] = 0.3f + 0.9f * r * r
            alpha[i] = 0.3f + 0.6f * rng.next()
            val t = rng.next()
            tint[i] = when {
                t < 0.82f -> 0
                t < 0.94f -> 1
                else -> 2
            }
        }
    }
}

private val STARS = Stars(STAR_COUNT)

/** The two skies the rewind runs between, with the clock instants they belong to. */
private class RewindSky(val now: ChartRecord, val birth: ChartRecord, val nowMillis: Long, val birthMillis: Long)

/**
 * The ritual first run. Calls [onDone] with the saved profile when the person steps through
 * the door at the end of the rewind. The profile is saved by the birth form itself.
 */
@Composable
fun ArrivalScreen(onDone: (BirthProfile) -> Unit) {
    val context = LocalContext.current
    val reduced = remember { !ValueAnimator.areAnimatorsEnabled() }
    var step by remember { mutableIntStateOf(1) }
    var profile by remember { mutableStateOf<BirthProfile?>(null) }

    Crossfade(targetState = step, animationSpec = tween(if (reduced) 150 else 700), label = "arrival") { s ->
        when (s) {
            1 -> Invocation(reduced, onBegin = { step = 2 })
            2 -> BirthScreen(existing = null, onCast = { profile = it; step = 3 })
            else -> {
                val p = profile ?: ProfileStore.load(context)
                if (p == null) {
                    LaunchedEffect(Unit) { step = 2 }
                } else {
                    Rewind(p, reduced, onEnter = { onDone(p) })
                }
            }
        }
    }
}

/* ------------------------------------ Step 1 ------------------------------------ */

@Composable
private fun Invocation(reduced: Boolean, onBegin: () -> Unit) {
    var shownOne by remember { mutableIntStateOf(if (reduced) LINE_ONE.length else 0) }
    var shownTwo by remember { mutableIntStateOf(if (reduced) LINE_TWO.length else 0) }
    var ready by remember { mutableStateOf(reduced) }

    LaunchedEffect(reduced) {
        if (reduced) return@LaunchedEffect
        delay(700)
        while (shownOne < LINE_ONE.length) {
            delay(TYPE_MS)
            shownOne++
        }
        delay(1000)
        while (shownTwo < LINE_TWO.length) {
            delay(TYPE_MS)
            shownTwo++
        }
        delay(500)
        ready = true
    }

    Box(
        Modifier
            .fillMaxSize()
            .background(KshanaColors.Night)
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                // A tap finishes the typing; the words are the same.
                shownOne = LINE_ONE.length
                shownTwo = LINE_TWO.length
                ready = true
            },
    ) {
        Column(Modifier.align(Alignment.Center).padding(horizontal = 32.dp)) {
            Text(
                LINE_ONE.take(shownOne),
                fontFamily = KshanaFonts.serif, fontSize = 24.sp, lineHeight = 34.sp, color = KshanaColors.TextSoft,
                minLines = 2,
            )
            Spacer(Modifier.height(28.dp))
            Text(
                LINE_TWO.take(shownTwo),
                style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text,
                minLines = 1,
            )
        }
        AnimatedVisibility(
            visible = ready,
            enter = fadeIn(tween(if (reduced) 150 else 600)),
            modifier = Modifier.align(Alignment.BottomCenter).padding(horizontal = 24.dp, vertical = 40.dp),
        ) {
            GoldButton("BEGIN", onClick = onBegin, modifier = Modifier.fillMaxWidth())
        }
    }
}

/* ------------------------------------ Step 3 ------------------------------------ */

@Composable
private fun Rewind(profile: BirthProfile, reduced: Boolean, onEnter: () -> Unit) {
    val context = LocalContext.current
    val zone = remember(profile) { profile.birthZone() }
    var sky by remember(profile) { mutableStateOf<RewindSky?>(null) }
    var error by remember(profile) { mutableStateOf<String?>(null) }
    val progress = remember(profile) { mutableFloatStateOf(if (reduced) 1f else 0f) }
    var done by remember(profile) { mutableStateOf(false) }

    // Both skies over the birth place, off the main thread: now, and the birth instant.
    LaunchedEffect(profile) {
        try {
            val engine = EngineProvider.get(context)
            val settings = ProfileStore.settings(context)
            sky = withContext(Dispatchers.Default) {
                val nowMillis = System.currentTimeMillis()
                val now = Instants.now()
                val birth = profile.birthInstant()
                val a = engine.chart(now, profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
                val b = engine.chart(birth, profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
                RewindSky(a, b, nowMillis, profile.birthLocal().toInstant().toEpochMilli())
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
            done = true
        }
    }

    // The rewind clock: 0 at now, 1 at the birth instant, over six seconds of frames.
    LaunchedEffect(sky, reduced) {
        if (sky == null) return@LaunchedEffect
        if (reduced) {
            progress.floatValue = 1f
            done = true
            return@LaunchedEffect
        }
        delay(HOLD_BEFORE_MS)
        val start = withFrameNanos { it }
        while (true) {
            val t = withFrameNanos { ((it - start) / 1e6 / REWIND_MILLIS).toFloat() }
            if (t >= 1f) break
            progress.floatValue = t
        }
        progress.floatValue = 1f
        done = true
    }

    Box(Modifier.fillMaxSize().background(KshanaColors.Night)) {
        RewindCanvas(sky, profile, progress, Modifier.matchParentSize())

        Column(
            Modifier.align(Alignment.TopStart).padding(horizontal = 24.dp).padding(top = 48.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text("THE SKY OVER ${profile.placeName.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            if (sky == null && error == null) {
                Text("Computing the sky over ${profile.placeName}.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            } else {
                RewindClock(sky, zone, progress)
            }
        }

        Column(
            Modifier.align(Alignment.BottomCenter).padding(horizontal = 24.dp).padding(bottom = 40.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            AnimatedVisibility(visible = done, enter = fadeIn(tween(if (reduced) 150 else 900))) {
                Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    val problem = error
                    if (problem != null) {
                        Text("The sky cannot be drawn now: $problem", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Sky)
                    }
                    Text(
                        "This is the sky when you arrived. Every number from here is computed from it.",
                        fontFamily = KshanaFonts.serif, fontSize = 20.sp, lineHeight = 28.sp, color = KshanaColors.Text,
                    )
                    GoldButton("ENTER", onClick = onEnter, modifier = Modifier.fillMaxWidth())
                }
            }
        }
    }
}

/** "34 y 211 d 05:12:33" for a span in milliseconds. */
private fun spanText(millis: Long): String {
    val total = abs(millis) / 1000L
    val year = 31_556_952L // 365.2425 days
    val years = total / year
    val days = (total % year) / 86_400L
    val rest = total % 86_400L
    return "%d y %d d %02d:%02d:%02d".format(years, days, rest / 3600L, (rest % 3600L) / 60L, rest % 60L)
}

/** The only composable that recomposes every frame: the instant shown and the span still to rewind. */
@Composable
private fun RewindClock(sky: RewindSky?, zone: ZoneId, progress: FloatState) {
    if (sky == null) return
    val e = ease(progress.floatValue)
    val span = sky.nowMillis - sky.birthMillis
    val at = sky.nowMillis - (span * e.toDouble()).toLong()
    Text(
        Instant.ofEpochMilli(at).atZone(zone).format(STAMP),
        fontFamily = KshanaFonts.mono, fontSize = 22.sp, color = KshanaColors.Sky,
    )
    Text(
        "${spanText(at - sky.birthMillis)} to go",
        fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Muted,
    )
}

/** One body on the ring: where it is now, how far it slides (the short way), and how it is drawn. */
private class Slide(
    val name: String,
    val fromLon: Double,
    val deltaLon: Double,
    val color: Color,
    val radius: Float,
    val glyphIndex: Int,
)

/**
 * The sky itself: stars, the ecliptic ring with its sign ticks, the horizon and the ground,
 * and the bodies sliding along the ring from now to the birth instant as [progress] runs 0 → 1.
 * Everything is laid out once in the cache; each frame reads one float and draws.
 */
@Composable
private fun RewindCanvas(sky: RewindSky?, profile: BirthProfile, progress: FloatState, modifier: Modifier) {
    val textMeasurer = rememberTextMeasurer()
    val labelStyle = remember { TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp) }
    val southern = profile.latitude < 0.0
    val description = if (sky == null) "The sky over ${profile.placeName}" else "The sky over ${profile.placeName} rewinding to the moment you arrived"

    Canvas(
        modifier
            .clipToBounds()
            .semantics { contentDescription = description }
            .drawWithCache {
                val w = size.width
                val h = size.height
                if (w <= 0f || h <= 0f) return@drawWithCache onDrawBehind { }
                val skies = sky
                val hasSky = skies != null

                // ---- layout ----
                val horizonY = h * HORIZON_FRACTION
                val cx = w * 0.5f
                val cy = horizonY
                val rx = w * 0.46f
                val ry = h * 0.22f
                val dir = if (southern) -1f else 1f
                val hair = 1.dp.toPx()

                val skyBrush = Brush.verticalGradient(0f to lerp(KshanaColors.Night, DAY_BLUE, 0.25f), 0.7f to KshanaColors.Night, startY = 0f, endY = h)
                val groundBrush = Brush.verticalGradient(
                    0f to KshanaColors.LineSoft.copy(alpha = 0.88f), 1f to KshanaColors.Night.copy(alpha = 0.96f),
                    startY = horizonY, endY = h,
                )
                val groundTopLeft = Offset(0f, horizonY)
                val groundSize = Size(w, h - horizonY)
                val horizonStart = Offset(0f, horizonY)
                val horizonEnd = Offset(w, horizonY)

                // ---- stars above the horizon; the field slides with the Ascendant ----
                val starX = FloatArray(STAR_COUNT) { STARS.x[it] * w }
                val starY = FloatArray(STAR_COUNT) { STARS.y[it] * horizonY }
                val starR = FloatArray(STAR_COUNT) { STARS.radius[it].dp.toPx() }

                // ---- the ring ----
                val ringStroke = Stroke(hair)
                val ringTopLeft = Offset(cx - rx, cy - ry)
                val ringSize = Size(2f * rx, 2f * ry)
                val ascFrom = skies?.now?.cuspsTropical?.firstOrNull() ?: 0.0
                val ascTo = skies?.birth?.cuspsTropical?.firstOrNull() ?: ascFrom
                val ascDelta = shortWay(ascFrom, ascTo)
                val starShift = -(ascDelta / 360.0).toFloat() * w * 0.5f
                val tickIn = 3.dp.toPx()
                val tickOut = 5.dp.toPx()
                val glyphOut = 14.dp.toPx()
                val signGlyphs = Array(12) { textMeasurer.measure(SIGN_GLYPHS[it], labelStyle) }
                val ascLabels = Array(12) { textMeasurer.measure("↑ Asc ${SIGN_GLYPHS[it]}", labelStyle) }
                val ascGap = 6.dp.toPx()
                val ascLift = 10.dp.toPx()
                val ascTickUp = 8.dp.toPx()
                val ascTickDown = 4.dp.toPx()
                val ascTickWidth = 1.5.dp.toPx()

                // ---- the bodies ----
                val labelGap = 3.dp.toPx()
                val glyphs = Array(DRAWN_BODIES.size) { textMeasurer.measure(BODY_GLYPHS[DRAWN_BODIES[it]] ?: "", labelStyle) }
                val slides: Array<Slide> = if (skies == null) emptyArray() else DRAWN_BODIES.mapIndexedNotNull { i, name ->
                    val from = skies.now.bodies.firstOrNull { it.name == name } ?: return@mapIndexedNotNull null
                    val to = skies.birth.bodies.firstOrNull { it.name == name } ?: return@mapIndexedNotNull null
                    val color = when (name) {
                        "Sun" -> KshanaColors.Gold
                        "Venus", "Jupiter" -> KshanaColors.Sky
                        else -> KshanaColors.Text
                    }
                    val radiusDp = when (name) {
                        "Sun" -> 7f
                        "Moon" -> 6f
                        "Venus" -> 2.4f
                        "Jupiter" -> 2.2f
                        "Mars" -> 1.9f
                        "Saturn" -> 1.7f
                        else -> 1.5f
                    }
                    Slide(name, from.tropical, shortWay(from.tropical, to.tropical), color, radiusDp.dp.toPx(), i)
                }.toTypedArray()
                val haloR = 32.dp.toPx()
                val sunHalo = Brush.radialGradient(
                    0f to KshanaColors.Gold.copy(alpha = 0.55f),
                    0.3f to KshanaColors.Gold.copy(alpha = 0.14f),
                    1f to KshanaColors.Gold.copy(alpha = 0f),
                    center = Offset.Zero, radius = haloR,
                )
                val markStroke = Stroke(hair)
                val moonRim = Stroke(0.5.dp.toPx().coerceAtLeast(1f))

                fun theta(asc: Double, lon: Double): Float = (PI + Math.toRadians(norm360(asc - lon))).toFloat()
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

                onDrawBehind {
                    // The only state read per frame.
                    val e = ease(progress.floatValue)
                    val shift = starShift * e
                    val asc = ascFrom + ascDelta * e

                    drawRect(skyBrush)
                    for (i in 0 until STAR_COUNT) {
                        val x = ((starX[i] + shift) % w + w) % w
                        val c = when (STARS.tint[i]) {
                            1 -> KshanaColors.Sky
                            2 -> KshanaColors.Gold
                            else -> KshanaColors.Text
                        }
                        drawCircle(c, starR[i], Offset(x, starY[i]), alpha = STARS.alpha[i])
                    }

                    drawOval(KshanaColors.Gold, ringTopLeft, ringSize, alpha = 0.35f, style = ringStroke)
                    if (hasSky) {
                        for (i in 0 until 12) {
                            val th = theta(asc, i * 30.0)
                            val px = ringX(th)
                            val py = ringY(th)
                            val nx = normalX(th)
                            val ny = normalY(th)
                            drawLine(KshanaColors.Gold, Offset(px - nx * tickIn, py - ny * tickIn), Offset(px + nx * tickOut, py + ny * tickOut), strokeWidth = hair, alpha = 0.5f)
                            val thm = theta(asc, i * 30.0 + 15.0)
                            val g = signGlyphs[i]
                            drawText(
                                g, color = KshanaColors.Muted,
                                topLeft = Offset(ringX(thm) + normalX(thm) * glyphOut - g.size.width / 2f, ringY(thm) + normalY(thm) * glyphOut - g.size.height / 2f),
                                alpha = 0.85f,
                            )
                        }
                    }
                    drawRect(groundBrush, groundTopLeft, groundSize)
                    drawLine(KshanaColors.Line, horizonStart, horizonEnd, strokeWidth = hair, alpha = 0.9f)

                    if (hasSky) {
                        for (s in slides) {
                            val lon = s.fromLon + s.deltaLon * e
                            val above = norm360(asc - lon) <= 180.0
                            val th = theta(asc, lon)
                            val x = ringX(th)
                            val y = ringY(th)
                            val g = glyphs[s.glyphIndex]
                            when (s.name) {
                                "Sun" -> {
                                    translate(x, y) { drawCircle(sunHalo, haloR, Offset.Zero, alpha = if (above) 1f else 0.3f) }
                                    if (above) drawCircle(KshanaColors.Gold, s.radius, Offset(x, y))
                                    else drawCircle(KshanaColors.Gold, s.radius * 0.6f, Offset(x, y), alpha = 0.4f, style = markStroke)
                                    drawText(g, color = KshanaColors.Gold, topLeft = Offset(x + s.radius + labelGap, y - g.size.height / 2f), alpha = if (above) 0.9f else 0.4f)
                                }
                                "Moon" -> {
                                    if (above) {
                                        drawCircle(KshanaColors.Text, s.radius, Offset(x, y))
                                        drawCircle(KshanaColors.TextSoft, s.radius, Offset(x, y), alpha = 0.18f, style = moonRim)
                                    } else {
                                        drawCircle(KshanaColors.TextSoft, s.radius * 0.6f, Offset(x, y), alpha = 0.35f, style = markStroke)
                                    }
                                    drawText(g, color = KshanaColors.TextSoft, topLeft = Offset(x + s.radius + labelGap, y - g.size.height / 2f), alpha = if (above) 0.9f else 0.35f)
                                }
                                else -> {
                                    val a = if (above) 1f else 0.3f
                                    drawCircle(s.color, s.radius, Offset(x, y), alpha = a)
                                    drawText(g, color = s.color, topLeft = Offset(x + s.radius + labelGap, y - g.size.height / 2f), alpha = a * 0.85f)
                                }
                            }
                        }

                        // The Ascendant: a gold tick at the east end of the ring, with its sign.
                        val ascTh = theta(asc, asc)
                        val ascX = ringX(ascTh)
                        val ascY = ringY(ascTh)
                        drawLine(KshanaColors.Gold, Offset(ascX, ascY - ascTickUp), Offset(ascX, ascY + ascTickDown), strokeWidth = ascTickWidth)
                        val label = ascLabels[(norm360(asc) / 30.0).toInt() % 12]
                        val labelX = if (southern) ascX - ascGap - label.size.width else ascX + ascGap
                        drawText(label, color = KshanaColors.Gold, topLeft = Offset(labelX, ascY - ascLift - label.size.height))
                    }
                }
            },
    ) { }
}
