package com.kshana.app.ui.theme

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.material3.LocalContentColor
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.sin

/** The six tab icons. Each is still when its tab is not chosen and comes alive when it is. */
enum class TabGlyph { SUN, EYE, CRYSTAL, HEART, STAR, MORE }

/**
 * A tab icon drawn by hand so it can move: the sun shines, the eye looks about and blinks,
 * the crystal ball turns like a globe, the heart beats, the star turns and twinkles, and
 * the More lines ripple. Animation runs only for the chosen tab, so the bar costs nothing
 * when idle. Colour comes from the navigation bar (gold when chosen, muted otherwise).
 */
@Composable
fun AnimatedTabIcon(glyph: TabGlyph, selected: Boolean, label: String) {
    val color = LocalContentColor.current
    // 0 → 1 as the tab becomes chosen; drives a little pop and the glow.
    val on by animateFloatAsState(if (selected) 1f else 0f, tween(420, easing = FastOutSlowInEasing), label = "on")
    val t = rememberInfiniteTransition(label = "tab")
    val zero = remember { mutableFloat(0f) }
    // Each clock only ticks while chosen; otherwise a fixed 0.
    val spin = if (selected) t.animateFloat(0f, 1f, infiniteRepeatable(tween(9000, easing = LinearEasing)), label = "spin") else zero
    val pulse = if (selected) t.animateFloat(0f, 1f, infiniteRepeatable(tween(1600, easing = FastOutSlowInEasing), RepeatMode.Reverse), label = "pulse") else zero
    val turn = if (selected) t.animateFloat(0f, 1f, infiniteRepeatable(tween(3200, easing = LinearEasing)), label = "turn") else zero
    val beat = if (selected) t.animateFloat(1f, 1f, infiniteRepeatable(keyframes {
        durationMillis = 1300
        1f at 0; 1.18f at 120; 1f at 260; 1.12f at 380; 1f at 560; 1f at 1300
    }), label = "beat") else remember { mutableFloat(1f) }
    val blink = if (selected) t.animateFloat(1f, 1f, infiniteRepeatable(keyframes {
        durationMillis = 4200
        1f at 0; 1f at 3600; 0.08f at 3720; 1f at 3860; 1f at 4200
    }), label = "blink") else remember { mutableFloat(1f) }

    Canvas(Modifier.size(28.dp).semantics { contentDescription = label }) {
        val pop = 1f + 0.08f * on
        scale(pop) {
            if (on > 0f) {
                drawCircle(
                    Brush.radialGradient(listOf(color.copy(alpha = 0.22f * on * (0.7f + 0.3f * pulse.value)), Color.Transparent), center, size.minDimension * 0.62f),
                    radius = size.minDimension * 0.62f,
                )
            }
            when (glyph) {
                TabGlyph.SUN -> sun(color, spin.value, pulse.value, on)
                TabGlyph.EYE -> eye(color, turn.value, blink.value, on)
                TabGlyph.CRYSTAL -> crystal(color, turn.value, pulse.value, on)
                TabGlyph.HEART -> heart(color, beat.value)
                TabGlyph.STAR -> star(color, spin.value, pulse.value, on)
                TabGlyph.MORE -> more(color, turn.value, on)
            }
        }
    }
}

private class FixedFloat(override val value: Float) : State<Float>
private fun mutableFloat(v: Float): State<Float> = FixedFloat(v)

/**
 * A sun: a glowing disc ringed by pointed, flame-like rays (long and short in turn) that
 * sit just off the disc. Chosen, it shines: the rays turn slowly and stretch in a wave,
 * the glow breathes, and a ring of light ripples out from the disc.
 */
private fun DrawScope.sun(c: Color, spin: Float, pulse: Float, on: Float) {
    val r = size.minDimension / 2
    val core = r * 0.36f
    val pi = PI.toFloat()

    if (on > 0f) {
        // Corona: a soft halo that breathes.
        drawCircle(
            Brush.radialGradient(listOf(c.copy(alpha = 0.55f * on), c.copy(alpha = 0.18f * on), Color.Transparent), center, r * (0.8f + 0.15f * pulse)),
            radius = r * (0.8f + 0.15f * pulse),
        )
        // A ring of light leaving the disc, fading as it grows.
        val ripple = (spin * 6f) % 1f
        drawCircle(c.copy(alpha = (1f - ripple) * 0.45f * on), core + r * 0.55f * ripple, style = Stroke(width = r * 0.05f))
    }

    // Twelve tapered rays, long and short in turn.
    rotate(spin * 120f) {
        for (i in 0 until 12) {
            val a = i * 30f * pi / 180f
            val long = i % 2 == 0
            val wave = if (on > 0f) 0.5f + 0.5f * sin(2 * pi * (pulse + i / 12f)) else 0.5f
            val inner = core + r * 0.1f
            val outer = inner + r * ((if (long) 0.42f else 0.26f) + 0.12f * wave * on)
            val half = r * (if (long) 0.11f else 0.08f)
            val perp = a + pi / 2
            val p = Path().apply {
                moveTo(center.x + cos(a) * inner + cos(perp) * half, center.y + sin(a) * inner + sin(perp) * half)
                lineTo(center.x + cos(a) * outer, center.y + sin(a) * outer)
                lineTo(center.x + cos(a) * inner - cos(perp) * half, center.y + sin(a) * inner - sin(perp) * half)
                close()
            }
            drawPath(p, c.copy(alpha = if (long) 1f else 0.8f))
        }
    }

    // The disc: bright at the heart, its own colour at the rim.
    drawCircle(
        Brush.radialGradient(listOf(Color.White.copy(alpha = 0.55f + 0.35f * on), c, c), center.copy(x = center.x - core * 0.25f, y = center.y - core * 0.25f), core * 1.4f),
        radius = core,
    )
}

private fun DrawScope.eye(c: Color, turn: Float, blink: Float, on: Float) {
    val w = size.width; val h = size.height
    val open = blink
    val lid = Path().apply {
        moveTo(w * 0.06f, h / 2)
        quadraticBezierTo(w / 2, h / 2 - h * 0.46f * open, w * 0.94f, h / 2)
        quadraticBezierTo(w / 2, h / 2 + h * 0.46f * open, w * 0.06f, h / 2)
        close()
    }
    drawPath(lid, c, style = Stroke(width = w * 0.075f))
    // The iris drifts left and right as if reading.
    val dx = if (on > 0f) sin(2 * PI.toFloat() * turn) * w * 0.12f * on else 0f
    clipPath(lid) {
        drawCircle(c, w * 0.17f, Offset(w / 2 + dx, h / 2))
        drawCircle(Color.Black.copy(alpha = 0.55f), w * 0.07f, Offset(w / 2 + dx, h / 2))
        drawCircle(Color.White.copy(alpha = 0.8f), w * 0.03f, Offset(w / 2 + dx + w * 0.05f, h / 2 - h * 0.05f))
    }
}

private fun DrawScope.crystal(c: Color, turn: Float, pulse: Float, on: Float) {
    val w = size.width; val h = size.height
    val r = w * 0.36f
    val ball = Offset(w / 2, h * 0.42f)
    // The stand.
    val base = Path().apply {
        moveTo(w * 0.24f, h * 0.94f); lineTo(w * 0.76f, h * 0.94f)
        lineTo(w * 0.66f, h * 0.76f); lineTo(w * 0.34f, h * 0.76f); close()
    }
    drawPath(base, c)
    drawCircle(c, r, ball, style = Stroke(width = w * 0.07f))
    val clip = Path().apply { addOval(Rect(ball, r * 0.96f)) }
    clipPath(clip) {
        // Meridians slide across like a turning globe.
        for (k in 0 until 3) {
            val phase = (turn + k / 3f) % 1f
            // A meridian at longitude phase·π projects to an ellipse of half-width r·|sin|;
            // those on the near side are drawn brighter than those going round the back.
            val ew = abs(sin(PI.toFloat() * phase)) * r
            val front = phase < 0.5f
            drawOval(
                c.copy(alpha = if (front) 0.8f else 0.32f),
                topLeft = Offset(ball.x - ew, ball.y - r),
                size = Size(ew * 2, r * 2),
                style = Stroke(width = w * 0.04f),
            )
        }
        // Latitude lines.
        drawLine(c.copy(alpha = 0.6f), Offset(ball.x - r, ball.y), Offset(ball.x + r, ball.y), strokeWidth = w * 0.04f)
        drawOval(c.copy(alpha = 0.45f), Offset(ball.x - r * 0.86f, ball.y - r * 0.62f), Size(r * 1.72f, r * 0.5f), style = Stroke(width = w * 0.03f))
    }
    // The spark in the glass twinkles.
    val s = w * (0.07f + 0.05f * pulse * on)
    val sp = Offset(ball.x + r * 0.38f, ball.y - r * 0.38f)
    drawLine(c, Offset(sp.x - s, sp.y), Offset(sp.x + s, sp.y), strokeWidth = w * 0.045f, cap = StrokeCap.Round)
    drawLine(c, Offset(sp.x, sp.y - s), Offset(sp.x, sp.y + s), strokeWidth = w * 0.045f, cap = StrokeCap.Round)
}

private fun DrawScope.heart(c: Color, beat: Float) {
    val w = size.width; val h = size.height
    scale(beat) {
        val p = Path().apply {
            moveTo(w / 2, h * 0.88f)
            cubicTo(w * -0.05f, h * 0.55f, w * 0.12f, h * 0.06f, w / 2, h * 0.3f)
            cubicTo(w * 0.88f, h * 0.06f, w * 1.05f, h * 0.55f, w / 2, h * 0.88f)
            close()
        }
        drawPath(p, c)
    }
}

private fun DrawScope.star(c: Color, spin: Float, pulse: Float, on: Float) {
    val r = size.minDimension * 0.46f
    rotate(spin * 72f * 2f) {
        val p = Path()
        for (i in 0 until 10) {
            val rr = if (i % 2 == 0) r else r * 0.42f
            val a = (-90f + i * 36f) * PI.toFloat() / 180f
            val x = center.x + cos(a) * rr; val y = center.y + sin(a) * rr
            if (i == 0) p.moveTo(x, y) else p.lineTo(x, y)
        }
        p.close()
        drawPath(p, c)
    }
    if (on > 0f) {
        // Two tiny sparkles take turns.
        for ((k, at) in listOf(Offset(0.9f, 0.12f), Offset(0.1f, 0.85f)).withIndex()) {
            val tw = (if (k == 0) pulse else 1f - pulse) * on
            val o = Offset(size.width * at.x, size.height * at.y)
            val s = size.minDimension * 0.09f * tw
            drawLine(c, Offset(o.x - s, o.y), Offset(o.x + s, o.y), strokeWidth = size.minDimension * 0.05f, cap = StrokeCap.Round)
            drawLine(c, Offset(o.x, o.y - s), Offset(o.x, o.y + s), strokeWidth = size.minDimension * 0.05f, cap = StrokeCap.Round)
        }
    }
}

private fun DrawScope.more(c: Color, turn: Float, on: Float) {
    val w = size.width; val h = size.height
    for (i in 0 until 3) {
        val y = h * (0.26f + 0.24f * i)
        val wave = if (on > 0f) 0.5f + 0.5f * sin(2 * PI.toFloat() * (turn - i / 6f)) else 1f
        val len = w * (0.42f + 0.34f * wave)
        drawCircle(c, w * 0.06f, Offset(w * 0.12f, y))
        drawLine(c, Offset(w * 0.28f, y), Offset(w * 0.28f + len * 0.85f, y), strokeWidth = w * 0.09f, cap = StrokeCap.Round)
    }
}
