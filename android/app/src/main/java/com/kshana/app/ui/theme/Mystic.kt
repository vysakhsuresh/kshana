package com.kshana.app.ui.theme

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import java.time.LocalTime
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin
import kotlin.random.Random

/**
 * The night as the ground of every screen: a faint starfield that breathes, under a
 * sky whose hue follows the hour (rose at dawn, indigo by day, amber at dusk, deep
 * night after). Drawn once behind the whole app; every screen is transparent over it.
 */
@Composable
fun NightSky(modifier: Modifier = Modifier) {
    val stars = remember {
        val r = Random(2026)
        List(140) { Triple(r.nextFloat(), r.nextFloat(), 0.6f + r.nextFloat() * 1.6f) to r.nextFloat() }
    }
    val t by rememberInfiniteTransition(label = "sky").animateFloat(
        0f, 1f, infiniteRepeatable(tween(9000, easing = LinearEasing), RepeatMode.Restart), label = "twinkle",
    )
    val hour = remember { LocalTime.now().hour + LocalTime.now().minute / 60f }
    // The tint at the top of the sky by hour: a hint, never a colour the eye reads.
    val tint = when {
        hour in 4.5f..7f -> Color(0xFF3A2430)
        hour in 7f..16.5f -> Color(0xFF151B3A)
        hour in 16.5f..19f -> Color(0xFF3A2A18)
        else -> Color(0xFF0B0E22)
    }
    Canvas(modifier.fillMaxSize()) {
        drawRect(KshanaColors.Night)
        drawRect(
            Brush.verticalGradient(0f to tint.copy(alpha = 0.55f), 0.45f to Color.Transparent),
        )
        drawCircle(
            Brush.radialGradient(listOf(KshanaColors.Gold.copy(alpha = 0.05f), Color.Transparent), center = Offset(size.width * 0.85f, size.height * 0.08f), radius = size.width * 0.9f),
            radius = size.width * 0.9f,
            center = Offset(size.width * 0.85f, size.height * 0.08f),
        )
        for ((p, phase) in stars) {
            val (fx, fy, r) = p
            val a = 0.25f + 0.45f * (0.5f + 0.5f * sin(2f * PI.toFloat() * (t + phase)))
            drawCircle(KshanaColors.Text.copy(alpha = a * 0.55f), r, Offset(fx * size.width, fy * size.height))
        }
    }
}

/** A surface of glass over the sky: translucent, with a hairline that catches gold at the top. */
fun Modifier.glass(radius: Dp = 18.dp): Modifier {
    val shape = RoundedCornerShape(radius)
    return this
        .clip(shape)
        .background(KshanaColors.Surface.copy(alpha = 0.78f))
        .border(
            1.dp,
            Brush.verticalGradient(listOf(KshanaColors.Gold.copy(alpha = 0.28f), KshanaColors.Line.copy(alpha = 0.55f), KshanaColors.Line.copy(alpha = 0.25f))),
            shape,
        )
}

/** A soft halo behind a ring or a button, the colour of its light. */
fun Modifier.halo(color: Color, radiusFactor: Float = 0.75f, alpha: Float = 0.22f): Modifier = drawBehind {
    val r = size.minDimension * radiusFactor
    drawCircle(Brush.radialGradient(listOf(color.copy(alpha = alpha), Color.Transparent), center = center, radius = r), radius = r, center = center)
}

/**
 * The loader: a wheel of twenty-seven points (the nakshatras) turning slowly, one of
 * them lit. Used wherever the engine is at work; never a spinner.
 */
@Composable
fun StarWheel(size: Dp = 56.dp, modifier: Modifier = Modifier) {
    val t by rememberInfiniteTransition(label = "wheel").animateFloat(
        0f, 1f, infiniteRepeatable(tween(5400, easing = LinearEasing), RepeatMode.Restart), label = "turn",
    )
    Box(modifier) {
        Canvas(Modifier.fillMaxSize()) {
            val r = this.size.minDimension / 2f - 4.dp.toPx()
            val c = center
            drawCircle(KshanaColors.Line.copy(alpha = 0.5f), r, c, style = Stroke(1f))
            val lit = (t * 27f).toInt() % 27
            for (i in 0 until 27) {
                val a = -PI.toFloat() / 2f + 2f * PI.toFloat() * i / 27f
                val p = Offset(c.x + r * cos(a), c.y + r * sin(a))
                val d = ((i - lit + 27) % 27)
                val alpha = when (d) { 0 -> 1f; 1 -> 0.6f; 2 -> 0.35f; else -> 0.15f }
                drawCircle(if (d == 0) KshanaColors.Gold else KshanaColors.Text, if (d == 0) 2.8f else 1.6f, p, alpha = alpha)
            }
        }
    }
}

/** A breathing alpha for a number that counts down. */
@Composable
fun breath(): Float {
    val a by rememberInfiniteTransition(label = "breath").animateFloat(
        0.72f, 1f, infiniteRepeatable(tween(2600), RepeatMode.Reverse), label = "alpha",
    )
    return a
}
