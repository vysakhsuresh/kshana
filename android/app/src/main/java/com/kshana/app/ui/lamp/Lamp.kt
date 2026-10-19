package com.kshana.app.ui.lamp

import android.animation.ValueAnimator
import android.content.Context
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.time.DateTimeException
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import kotlin.math.abs
import kotlin.math.sin
import com.kshana.app.ui.sky.KshanaFonts

/*
 * The lamp: the return mechanic without guilt. A diya on the Today screen is lit when the
 * app was opened since this solar day's sunrise and unlit otherwise. There is no count, no
 * streak and no history; the only two states are "lit" and "not yet today". The day is the
 * solar day the engine gives (sunrise to the next sunrise), so the caller passes the sunrise.
 */

private const val FLICKER_PERIOD_MS = 6667 // 0.15 Hz
private const val TWO_PI = 6.2831855f
private val TIME_ONLY = DateTimeFormatter.ofPattern("HH:mm:ss")

/** Lit or not for one solar day, keyed on that day's sunrise. */
object LampStore {
    private const val K_SUNRISE = "lamp.sunriseMillis"
    /** Two solutions of the same sunrise differ by milliseconds at most; two days by about a day. */
    private const val SAME_DAY_MILLIS = 3_600_000L

    /** True when the lamp was last lit for the solar day that began at [sunriseEpochMillis]. */
    fun litToday(context: Context, sunriseEpochMillis: Long): Boolean {
        val last = ProfileStore.prefs(context).getLong(K_SUNRISE, Long.MIN_VALUE)
        return last != Long.MIN_VALUE && abs(last - sunriseEpochMillis) < SAME_DAY_MILLIS
    }

    /** Lights the lamp for the solar day that began at [sunriseEpochMillis]; earlier days are forgotten. */
    fun light(context: Context, sunriseEpochMillis: Long) {
        ProfileStore.prefs(context).edit().putLong(K_SUNRISE, sunriseEpochMillis).apply()
    }
}

/** True when the system lets animators run (false under "Remove animations" or battery saver). */
private fun motionAllowed(): Boolean = ValueAnimator.areAnimatorsEnabled()

/**
 * A small diya: a bowl and, when [lit], a flame that flickers at 0.15 Hz. Tapping an unlit
 * lamp calls [onLight] and the lamp springs once (under 300 ms); a lit lamp does nothing on
 * tap. Under reduced motion the flame stands still and the spring is skipped. 56 dp if [modifier]
 * gives no size.
 */
@Composable
fun Lamp(lit: Boolean, onLight: () -> Unit, modifier: Modifier = Modifier) {
    val motion = remember { motionAllowed() }
    val still = remember { mutableFloatStateOf(0.25f) }
    val flicker: State<Float> = if (lit && motion) {
        val transition = rememberInfiniteTransition(label = "flame")
        transition.animateFloat(
            initialValue = 0f,
            targetValue = 1f,
            animationSpec = infiniteRepeatable(tween(FLICKER_PERIOD_MS, easing = LinearEasing), RepeatMode.Restart),
            label = "flicker",
        )
    } else {
        still
    }
    val scale = remember { Animatable(1f) }
    val scope = rememberCoroutineScope()
    val description = if (lit) "The lamp is lit for this day" else "Light the lamp for this day"

    Canvas(
        modifier
            .size(56.dp)
            .graphicsLayer {
                val s = scale.value
                scaleX = s
                scaleY = s
            }
            .semantics { contentDescription = description }
            .clickable(enabled = !lit, role = Role.Button) {
                onLight()
                if (motion) {
                    scope.launch {
                        scale.snapTo(0.82f)
                        scale.animateTo(1f, spring(dampingRatio = Spring.DampingRatioLowBouncy, stiffness = Spring.StiffnessMedium))
                    }
                }
            }
            .drawWithCache {
                val w = size.width
                val h = size.height
                val cx = w / 2f
                val hair = 1.dp.toPx()

                // The bowl: the lower half of an oval, with its rim as a thin ellipse.
                val bowlW = w * 0.72f
                val bowlH = h * 0.30f
                val rimY = h * 0.66f
                val bowlTopLeft = Offset(cx - bowlW / 2f, rimY - bowlH / 2f)
                val bowlSize = Size(bowlW, bowlH)
                val rimH = h * 0.07f
                val rimTopLeft = Offset(cx - bowlW / 2f, rimY - rimH / 2f)
                val rimSize = Size(bowlW, rimH)
                val rimStroke = Stroke(hair)
                val bowlBrush = Brush.verticalGradient(
                    0f to (if (lit) KshanaColors.Gold else KshanaColors.Line),
                    1f to (if (lit) KshanaColors.Gold.copy(alpha = 0.55f) else KshanaColors.LineSoft),
                    startY = rimY, endY = rimY + bowlH / 2f,
                )

                // The wick stands on the rim; the flame is a teardrop above it, scaled per frame.
                val wickTop = Offset(cx, rimY - h * 0.06f)
                val wickBottom = Offset(cx, rimY)
                val wickStroke = Stroke(1.5.dp.toPx(), cap = StrokeCap.Round)
                val flameBase = rimY - h * 0.05f
                val flameH = h * 0.36f
                val flameW = w * 0.11f
                val flame = Path().apply {
                    moveTo(cx, flameBase - flameH)
                    quadraticTo(cx + flameW * 1.6f, flameBase - flameH * 0.35f, cx, flameBase)
                    quadraticTo(cx - flameW * 1.6f, flameBase - flameH * 0.35f, cx, flameBase - flameH)
                    close()
                }
                val core = Path().apply {
                    moveTo(cx, flameBase - flameH * 0.55f)
                    quadraticTo(cx + flameW * 0.7f, flameBase - flameH * 0.2f, cx, flameBase)
                    quadraticTo(cx - flameW * 0.7f, flameBase - flameH * 0.2f, cx, flameBase - flameH * 0.55f)
                    close()
                }
                val flamePivot = Offset(cx, flameBase)
                val glowCentre = Offset(cx, flameBase - flameH * 0.45f)
                val glowR = h * 0.42f
                val glow = Brush.radialGradient(
                    0f to KshanaColors.Gold.copy(alpha = 0.40f),
                    0.5f to KshanaColors.Gold.copy(alpha = 0.10f),
                    1f to KshanaColors.Gold.copy(alpha = 0f),
                    center = glowCentre, radius = glowR,
                )

                onDrawBehind {
                    // The only state read per frame: the flicker clock.
                    val phase = flicker.value * TWO_PI
                    val breath = sin(phase)
                    val sway = sin(phase * 2.3f + 1f)

                    if (lit) drawCircle(glow, glowR, glowCentre, alpha = 0.75f + 0.25f * breath)
                    drawArc(bowlBrush, 0f, 180f, true, bowlTopLeft, bowlSize)
                    drawOval(if (lit) KshanaColors.Gold else KshanaColors.Line, rimTopLeft, rimSize, alpha = if (lit) 0.9f else 0.7f, style = rimStroke)
                    drawOval(KshanaColors.Night, rimTopLeft, rimSize, alpha = 0.35f)
                    drawLine(if (lit) KshanaColors.Night else KshanaColors.Muted, wickTop, wickBottom, strokeWidth = wickStroke.width, cap = StrokeCap.Round, alpha = if (lit) 0.6f else 0.5f)
                    if (lit) {
                        scale(scaleX = 1f - 0.05f * sway, scaleY = 1f + 0.07f * breath, pivot = flamePivot) {
                            drawPath(flame, KshanaColors.Gold, alpha = 0.95f)
                            drawPath(core, KshanaColors.Text, alpha = 0.85f)
                        }
                    }
                }
            },
    ) { }
}

/** The sunrise, or the local midnight when the Sun does not rise, that begins this day. */
private class SolarDay(val startMillis: Long, val sunriseText: String?)

/**
 * The lamp for this solar day, for the Today screen: the day's sunrise is read from the
 * engine's panchanga for [here], the lamp shows whether it is lit for that day, and one tap
 * lights it. The sentence beside it is the whole state: lit, or not yet today.
 */
@Composable
fun TodayLamp(profile: BirthProfile, here: Place, settings: EngineSettings, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val zone = remember(here, profile) {
        try { ZoneId.of(here.zoneId) } catch (e: DateTimeException) { profile.zone() }
    }
    var day by remember(here, settings) { mutableStateOf<SolarDay?>(null) }
    var lit by remember { mutableStateOf(false) }

    LaunchedEffect(here, settings) {
        val solved: SolarDay = try {
            val engine = EngineProvider.get(context)
            withContext(Dispatchers.Default) {
                val detail = engine.panchangaDetail(Instants.now(), here.latitude, here.longitude, settings.ayanamsa)
                val rise = detail.sunrise
                if (rise != null) {
                    val utc = Instants.toLocal(rise, ZoneOffset.UTC)
                    SolarDay(utc.toInstant().toEpochMilli(), utc.withZoneSameInstant(zone).format(TIME_ONLY))
                } else {
                    SolarDay(LocalDate.now(zone).atStartOfDay(zone).toInstant().toEpochMilli(), null)
                }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            SolarDay(LocalDate.now(zone).atStartOfDay(zone).toInstant().toEpochMilli(), null)
        }
        day = solved
        lit = LampStore.litToday(context, solved.startMillis)
    }

    val d = day
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        Lamp(
            lit = lit,
            onLight = {
                if (d != null) {
                    LampStore.light(context, d.startMillis)
                    lit = true
                }
            },
        )
        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(
                if (lit) "The lamp is lit for this day" else "Light the lamp for this day",
                fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text,
            )
            Text(
                when {
                    d == null -> "Reading this day's sunrise."
                    d.sunriseText != null -> "This day runs from sunrise, ${d.sunriseText}, to the next sunrise."
                    else -> "The Sun does not rise here today; this day runs from midnight."
                },
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                modifier = Modifier.padding(end = 8.dp),
            )
        }
    }
}
