package com.kshana.app.ui.theme

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathOperation
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

/**
 * Old Indian things drawn in gold line, each with one small movement: the lamp's flame
 * sways, the cowries are cast, the sundial's shadow walks, the temple bell swings, the
 * scroll unrolls, the chariot wheel turns, the hourglass runs and turns over, the lotus
 * opens, the banyan sways and lets a leaf fall, the moon waxes, the eye looks and blinks,
 * the palm leaf is written on, the scales weigh, the kalasha's leaves stir, the star
 * twinkles, the yantra's triangles turn, the planets orbit, the conch sounds, the kolam
 * is drawn, the sun turns and the arrow finds the centre.
 */
enum class Ancient { LAMP, COWRIES, SUNDIAL, BELL, SCROLL, WHEEL, HOURGLASS, LOTUS, TREE, MOON, EYE, LEAF, SCALES, KALASHA, STAR, YANTRA, ORBIT, CONCH, KOLAM, SUN, BOW }

/**
 * One slow clock (0 → 1 every 12 seconds) shared by every icon on a screen. Icons read it
 * only while drawing, so the cards never recompose; only their pictures redraw.
 */
@Composable
fun rememberAncientClock(): State<Float> =
    rememberInfiniteTransition(label = "ancient").animateFloat(0f, 1f, infiniteRepeatable(tween(12_000, easing = LinearEasing)), label = "clock")

/** An ancient icon. [seed] shifts its phase so neighbouring icons don't move in step. */
@Composable
fun AncientIcon(glyph: Ancient, clock: State<Float>, modifier: Modifier = Modifier, color: Color = KshanaColors.Gold, seed: Int = 0) {
    Canvas(modifier) {
        val t = (clock.value + seed * 0.137f) % 1f
        // A soft ground glow so the line art sits in light.
        drawCircle(Brush.radialGradient(listOf(color.copy(alpha = 0.16f), Color.Transparent), center, size.minDimension * 0.6f), size.minDimension * 0.6f)
        when (glyph) {
            Ancient.LAMP -> lamp(color, t)
            Ancient.COWRIES -> cowries(color, t)
            Ancient.SUNDIAL -> sundial(color, t)
            Ancient.BELL -> bell(color, t)
            Ancient.SCROLL -> scroll(color, t)
            Ancient.WHEEL -> wheel(color, t)
            Ancient.HOURGLASS -> hourglass(color, t)
            Ancient.LOTUS -> lotus(color, t)
            Ancient.TREE -> tree(color, t)
            Ancient.MOON -> moon(color, t)
            Ancient.EYE -> eye(color, t)
            Ancient.LEAF -> leaf(color, t)
            Ancient.SCALES -> scales(color, t)
            Ancient.KALASHA -> kalasha(color, t)
            Ancient.STAR -> star(color, t)
            Ancient.YANTRA -> yantra(color, t)
            Ancient.ORBIT -> orbit(color, t)
            Ancient.CONCH -> conch(color, t)
            Ancient.KOLAM -> kolam(color, t)
            Ancient.SUN -> sun(color, t)
            Ancient.BOW -> bow(color, t)
        }
    }
}

private const val TAU = (2 * PI).toFloat()
private fun wave(t: Float, k: Float, phase: Float = 0f) = sin(TAU * (t * k + phase))
private fun frac(x: Float) = x - kotlin.math.floor(x)
private fun ease(x: Float): Float { val c = x.coerceIn(0f, 1f); return c * c * (3 - 2 * c) }
private fun DrawScope.s() = size.minDimension
private fun DrawScope.p(x: Float, y: Float) = Offset(x * s(), y * s())
private fun DrawScope.line(c: Color) = Stroke(width = s() * 0.055f, cap = StrokeCap.Round)
private fun DrawScope.thin(c: Color) = Stroke(width = s() * 0.035f, cap = StrokeCap.Round)

/** A clay diya with a living flame. */
private fun DrawScope.lamp(c: Color, t: Float) {
    val f = 0.5f + 0.3f * wave(t, 23f) + 0.2f * wave(t, 37f)
    val sway = 0.035f * wave(t, 11f)
    drawCircle(Brush.radialGradient(listOf(c.copy(alpha = 0.45f), Color.Transparent), p(0.5f, 0.4f), s() * (0.32f + 0.05f * f)), s() * (0.32f + 0.05f * f), p(0.5f, 0.4f))
    val bowl = Path().apply {
        moveTo(0.1f * s(), 0.6f * s()); lineTo(0.78f * s(), 0.6f * s()); lineTo(0.93f * s(), 0.53f * s())
        quadraticBezierTo(0.82f * s(), 0.86f * s(), 0.5f * s(), 0.86f * s())
        quadraticBezierTo(0.16f * s(), 0.86f * s(), 0.1f * s(), 0.6f * s()); close()
    }
    drawPath(bowl, c.copy(alpha = 0.22f)); drawPath(bowl, c, style = line(c))
    val h = 0.27f + 0.05f * f
    val flame = Path().apply {
        moveTo(0.5f * s(), 0.58f * s())
        cubicTo((0.38f) * s(), 0.5f * s(), (0.44f + sway * 0.5f) * s(), 0.4f * s(), (0.5f + sway) * s(), (0.58f - h) * s())
        cubicTo((0.56f + sway * 0.5f) * s(), 0.4f * s(), 0.62f * s(), 0.5f * s(), 0.5f * s(), 0.58f * s()); close()
    }
    drawPath(flame, c)
    translate(left = 0f, top = s() * 0.05f) { scaled(0.5f, flame, Color.White.copy(alpha = 0.55f)) }
}

private fun DrawScope.scaled(k: Float, path: Path, color: Color) {
    val pivot = p(0.5f, 0.56f)
    scale(k, k, pivot) { drawPath(path, color) }
}

/** Three cowrie shells, cast in turn. */
private fun DrawScope.cowries(c: Color, t: Float) {
    for (i in 0 until 3) {
        val x = 0.22f + 0.28f * i
        val hop = abs(sin(TAU * (t * 3f + i / 3f) / 2f))
        val y = 0.62f - 0.2f * hop * hop
        rotate(-18f + 18f * i + 25f * hop, p(x, y)) {
            val w = 0.2f * s(); val h = 0.3f * s()
            drawOval(c.copy(alpha = 0.2f), Offset(x * s() - w / 2, y * s() - h / 2), Size(w, h))
            drawOval(c, Offset(x * s() - w / 2, y * s() - h / 2), Size(w, h), style = thin(c))
            drawLine(c, p(x, y - 0.1f), p(x, y + 0.1f), s() * 0.03f, StrokeCap.Round)
            for (k in -1..1) {
                val yy = y + 0.05f * k
                drawLine(c, p(x - 0.035f, yy), p(x - 0.012f, yy), s() * 0.022f, StrokeCap.Round)
                drawLine(c, p(x + 0.012f, yy), p(x + 0.035f, yy), s() * 0.022f, StrokeCap.Round)
            }
        }
    }
}

/** A sundial whose shadow walks with the sun. */
private fun DrawScope.sundial(c: Color, t: Float) {
    val o = p(0.5f, 0.72f); val r = 0.4f * s()
    drawArc(c, 180f, 180f, false, Offset(o.x - r, o.y - r), Size(2 * r, 2 * r), style = line(c))
    drawLine(c, Offset(o.x - r, o.y), Offset(o.x + r, o.y), s() * 0.055f, StrokeCap.Round)
    for (k in 1..5) {
        val a = (180f + 30f * k) * PI.toFloat() / 180f
        drawLine(c.copy(alpha = 0.7f), Offset(o.x + cos(a) * r * 0.82f, o.y + sin(a) * r * 0.82f), Offset(o.x + cos(a) * r * 0.95f, o.y + sin(a) * r * 0.95f), s() * 0.03f, StrokeCap.Round)
    }
    val u = 0.5f + 0.5f * wave(t, 2f)
    val sunA = (200f + 140f * u) * PI.toFloat() / 180f
    val shA = (540f - (200f + 140f * u)) * PI.toFloat() / 180f
    drawLine(c.copy(alpha = 0.55f), o, Offset(o.x + cos(shA) * r * 0.72f, o.y + sin(shA) * r * 0.72f), s() * 0.07f, StrokeCap.Round)
    drawLine(c, o, Offset(o.x, o.y - r * 0.5f), s() * 0.05f, StrokeCap.Round)
    drawCircle(c, s() * 0.06f, Offset(o.x + cos(sunA) * r * 1.08f, o.y + sin(sunA) * r * 1.08f))
}

/** A temple bell, swinging, with its ring spreading. */
private fun DrawScope.bell(c: Color, t: Float) {
    val swing = 16f * wave(t, 4f)
    val pivot = p(0.5f, 0.1f)
    val loud = abs(wave(t, 4f))
    for (side in listOf(-1f, 1f)) {
        drawArc(c.copy(alpha = 0.5f * loud), if (side < 0) 150f else -30f, 60f, false, Offset((0.5f + side * 0.05f) * s() - 0.42f * s(), 0.5f * s() - 0.42f * s()), Size(0.84f * s(), 0.84f * s()), style = thin(c))
    }
    rotate(swing, pivot) {
        drawLine(c, pivot, p(0.5f, 0.2f), s() * 0.04f)
        drawCircle(c, s() * 0.045f, p(0.5f, 0.22f))
        val body = Path().apply {
            moveTo(0.4f * s(), 0.27f * s())
            cubicTo(0.36f * s(), 0.5f * s(), 0.3f * s(), 0.62f * s(), 0.2f * s(), 0.7f * s())
            lineTo(0.8f * s(), 0.7f * s())
            cubicTo(0.7f * s(), 0.62f * s(), 0.64f * s(), 0.5f * s(), 0.6f * s(), 0.27f * s()); close()
        }
        drawPath(body, c.copy(alpha = 0.22f)); drawPath(body, c, style = line(c))
        rotate(swing * 0.8f, p(0.5f, 0.6f)) { drawCircle(c, s() * 0.06f, p(0.5f, 0.79f)) }
    }
}

/** A scroll that unrolls and rolls up again. */
private fun DrawScope.scroll(c: Color, t: Float) {
    val u = ease(0.5f + 0.5f * wave(t, 1.5f))
    val half = 0.06f + 0.26f * u
    val l = 0.5f - half; val r = 0.5f + half
    drawRect(c.copy(alpha = 0.16f), p(l, 0.27f), Size((r - l) * s(), 0.46f * s()))
    drawLine(c, p(l, 0.27f), p(r, 0.27f), s() * 0.035f); drawLine(c, p(l, 0.73f), p(r, 0.73f), s() * 0.035f)
    for (k in 0 until 3) {
        val y = 0.39f + 0.11f * k
        if (r - l > 0.2f) drawLine(c.copy(alpha = 0.8f), p(l + 0.08f, y), p(r - 0.08f - 0.06f * (k % 2), y), s() * 0.03f, StrokeCap.Round)
    }
    for (x in listOf(l, r)) {
        drawRoundRect(c, p(x - 0.05f, 0.2f), Size(0.1f * s(), 0.6f * s()), androidx.compose.ui.geometry.CornerRadius(0.05f * s()))
    }
}

/** The Konark chariot wheel, turning. */
private fun DrawScope.wheel(c: Color, t: Float) {
    rotate(t * 360f) {
        val o = center
        drawCircle(c, 0.42f * s(), o, style = line(c))
        drawCircle(c.copy(alpha = 0.7f), 0.33f * s(), o, style = thin(c))
        for (k in 0 until 8) {
            val a = TAU * k / 8
            drawLine(c, Offset(o.x + cos(a) * 0.08f * s(), o.y + sin(a) * 0.08f * s()), Offset(o.x + cos(a) * 0.33f * s(), o.y + sin(a) * 0.33f * s()), s() * (if (k % 2 == 0) 0.05f else 0.03f), StrokeCap.Round)
        }
        for (k in 0 until 16) {
            val a = TAU * (k + 0.5f) / 16
            drawCircle(c, 0.02f * s(), Offset(o.x + cos(a) * 0.375f * s(), o.y + sin(a) * 0.375f * s()))
        }
        drawCircle(c, 0.08f * s(), o)
    }
}

/** An hourglass that runs, then turns over. */
private fun DrawScope.hourglass(c: Color, t: Float) {
    val ph = frac(t * 3f)
    val q = min(ph / 0.85f, 1f)
    val flip = if (ph > 0.85f) 180f * ease((ph - 0.85f) / 0.15f) else 0f
    rotate(flip) {
        drawLine(c, p(0.22f, 0.12f), p(0.78f, 0.12f), s() * 0.06f, StrokeCap.Round)
        drawLine(c, p(0.22f, 0.88f), p(0.78f, 0.88f), s() * 0.06f, StrokeCap.Round)
        val glass = Path().apply {
            moveTo(0.27f * s(), 0.16f * s()); lineTo(0.73f * s(), 0.16f * s()); lineTo(0.53f * s(), 0.5f * s())
            lineTo(0.73f * s(), 0.84f * s()); lineTo(0.27f * s(), 0.84f * s()); lineTo(0.47f * s(), 0.5f * s()); close()
        }
        drawPath(glass, c, style = thin(c))
        // Sand above: a triangle whose top falls as it empties.
        val top = 0.22f + 0.26f * q
        val hw = 0.21f * (0.48f - top) / 0.28f
        if (q < 1f) drawPath(Path().apply { moveTo((0.5f - hw) * s(), top * s()); lineTo((0.5f + hw) * s(), top * s()); lineTo(0.5f * s(), 0.48f * s()); close() }, c)
        // Sand below: a growing mound, and the stream while it runs.
        val h = 0.26f * q
        drawPath(Path().apply { moveTo(0.3f * s(), 0.81f * s()); lineTo(0.7f * s(), 0.81f * s()); lineTo(0.5f * s(), (0.81f - h) * s()); close() }, c)
        if (q in 0.02f..0.98f) drawLine(c, p(0.5f, 0.5f), p(0.5f, 0.81f - h), s() * 0.02f)
    }
}

/** A lotus that opens and closes on the water. */
private fun DrawScope.lotus(c: Color, t: Float) {
    val open = 0.5f + 0.5f * wave(t, 1.5f)
    val base = p(0.5f, 0.7f)
    for (k in -2..2) {
        val ang = k * (14f + 16f * open)
        val len = if (k == 0) 0.5f else 0.42f - 0.03f * abs(k)
        rotate(ang, base) {
            val petal = Path().apply {
                moveTo(base.x, base.y)
                quadraticBezierTo(base.x - 0.15f * s(), base.y - len * 0.55f * s(), base.x, base.y - len * s())
                quadraticBezierTo(base.x + 0.15f * s(), base.y - len * 0.55f * s(), base.x, base.y)
            }
            drawPath(petal, c.copy(alpha = 0.14f + 0.06f * (2 - abs(k))))
            drawPath(petal, c, style = thin(c))
        }
    }
    for (k in 0 until 2) {
        val y = 0.8f + 0.08f * k
        val path = Path().apply {
            moveTo(0.12f * s(), y * s())
            for (i in 1..8) { val x = 0.12f + 0.095f * i; lineTo(x * s(), (y + 0.02f * wave(t, 3f, i / 8f + k * 0.3f)) * s()) }
        }
        drawPath(path, c.copy(alpha = 0.6f - 0.25f * k), style = thin(c))
    }
}

/** A banyan swaying, letting a leaf go. */
private fun DrawScope.tree(c: Color, t: Float) {
    val base = p(0.5f, 0.9f)
    rotate(3f * wave(t, 2f), base) {
        drawLine(c, base, p(0.5f, 0.52f), s() * 0.08f, StrokeCap.Round)
        drawLine(c, p(0.5f, 0.64f), p(0.34f, 0.5f), s() * 0.045f, StrokeCap.Round)
        drawLine(c, p(0.5f, 0.6f), p(0.66f, 0.48f), s() * 0.045f, StrokeCap.Round)
        for (x in listOf(0.3f, 0.7f)) drawLine(c.copy(alpha = 0.5f), p(x, 0.5f), p(x, 0.82f), s() * 0.02f)
        for ((x, y, r) in listOf(Triple(0.3f, 0.42f, 0.17f), Triple(0.7f, 0.42f, 0.17f), Triple(0.5f, 0.3f, 0.21f))) {
            drawCircle(c.copy(alpha = 0.2f), r * s(), p(x, y)); drawCircle(c, r * s(), p(x, y), style = thin(c))
        }
    }
    val lf = frac(t * 2f)
    val lx = 0.72f + 0.08f * wave(lf, 2f); val ly = 0.5f + 0.42f * lf
    rotate(60f * wave(lf, 2f), p(lx, ly)) { drawOval(c.copy(alpha = 1f - lf), p(lx - 0.04f, ly - 0.02f), Size(0.08f * s(), 0.04f * s())) }
}

/** The moon waxing and waning among two stars. */
private fun DrawScope.moon(c: Color, t: Float) {
    val r = 0.3f * s(); val o = p(0.46f, 0.52f)
    drawCircle(c.copy(alpha = 0.18f), r, o)
    val k = 0.5f + 0.5f * cos(TAU * t)
    val disc = Path().apply { addOval(androidx.compose.ui.geometry.Rect(o, r)) }
    val shade = Path().apply { addOval(androidx.compose.ui.geometry.Rect(Offset(o.x + r * (0.35f + 1.6f * k), o.y - r * 0.12f), r)) }
    drawPath(Path.combine(PathOperation.Difference, disc, shade), c)
    for ((i, xy) in listOf(0.82f to 0.22f, 0.86f to 0.62f).withIndex()) {
        val tw = 0.5f + 0.5f * wave(t, 5f, i * 0.4f)
        sparkle(c.copy(alpha = 0.4f + 0.6f * tw), p(xy.first, xy.second), s() * (0.04f + 0.03f * tw))
    }
}

private fun DrawScope.sparkle(c: Color, o: Offset, r: Float) {
    drawLine(c, Offset(o.x - r, o.y), Offset(o.x + r, o.y), r * 0.45f, StrokeCap.Round)
    drawLine(c, Offset(o.x, o.y - r), Offset(o.x, o.y + r), r * 0.45f, StrokeCap.Round)
}

/** The seeing eye: it looks about and blinks. */
private fun DrawScope.eye(c: Color, t: Float) {
    val b = frac(t * 3f)
    val open = if (b > 0.9f) 1f - sin(PI.toFloat() * (b - 0.9f) / 0.1f) * 0.92f else 1f
    val lid = Path().apply {
        moveTo(0.08f * s(), 0.56f * s())
        quadraticBezierTo(0.5f * s(), (0.56f - 0.42f * open) * s(), 0.92f * s(), 0.56f * s())
        quadraticBezierTo(0.5f * s(), (0.56f + 0.42f * open) * s(), 0.08f * s(), 0.56f * s()); close()
    }
    drawPath(lid, c.copy(alpha = 0.15f)); drawPath(lid, c, style = line(c))
    if (open > 0.3f) {
        val ix = 0.5f + 0.13f * wave(t, 2f)
        drawCircle(c, 0.13f * s() * open, p(ix, 0.56f))
        drawCircle(KshanaColors.Night, 0.055f * s() * open, p(ix, 0.56f))
        drawCircle(Color.White.copy(alpha = 0.7f), 0.025f * s(), p(ix + 0.04f, 0.52f))
    }
    for (k in -1..1) drawLine(c.copy(alpha = 0.8f), p(0.5f + 0.17f * k, 0.22f + 0.03f * abs(k)), p(0.5f + 0.21f * k, 0.12f + 0.03f * abs(k)), s() * 0.035f, StrokeCap.Round)
}

/** A palm-leaf manuscript being written. */
private fun DrawScope.leaf(c: Color, t: Float) {
    val w = frac(t * 2f)
    for (k in 0 until 2) {
        rotate(-6f + 10f * k, center) {
            val y = 0.36f + 0.22f * k
            drawRoundRect(c.copy(alpha = 0.18f), p(0.1f, y), Size(0.8f * s(), 0.2f * s()), androidx.compose.ui.geometry.CornerRadius(0.1f * s()))
            drawRoundRect(c, p(0.1f, y), Size(0.8f * s(), 0.2f * s()), androidx.compose.ui.geometry.CornerRadius(0.1f * s()), style = thin(c))
            drawCircle(c, 0.025f * s(), p(0.3f, y + 0.1f))
            if (k == 1) for (ln in 0 until 2) {
                val part = ((w * 2f) - ln).coerceIn(0f, 1f)
                val yy = y + 0.07f + 0.07f * ln
                if (part > 0f) drawLine(c, p(0.38f, yy), p(0.38f + 0.44f * part, yy), s() * 0.025f, StrokeCap.Round)
            }
        }
    }
    drawLine(c.copy(alpha = 0.7f), p(0.3f, 0.3f), p(0.3f, 0.9f), s() * 0.02f)
}

/** The scales, weighing. */
private fun DrawScope.scales(c: Color, t: Float) {
    val tilt = 11f * wave(t, 1.5f)
    val pivot = p(0.5f, 0.22f)
    drawLine(c, pivot, p(0.5f, 0.86f), s() * 0.05f, StrokeCap.Round)
    drawLine(c, p(0.32f, 0.88f), p(0.68f, 0.88f), s() * 0.06f, StrokeCap.Round)
    drawCircle(c, 0.045f * s(), pivot)
    val a = tilt * PI.toFloat() / 180f
    for (side in listOf(-1f, 1f)) {
        val end = Offset(pivot.x + side * cos(a) * 0.36f * s(), pivot.y + side * sin(a) * 0.36f * s())
        drawLine(c, pivot, end, s() * 0.045f, StrokeCap.Round)
        val panY = end.y + 0.3f * s()
        drawLine(c.copy(alpha = 0.8f), end, Offset(end.x - 0.1f * s(), panY), s() * 0.022f)
        drawLine(c.copy(alpha = 0.8f), end, Offset(end.x + 0.1f * s(), panY), s() * 0.022f)
        drawArc(c, 0f, 180f, true, Offset(end.x - 0.13f * s(), panY - 0.07f * s()), Size(0.26f * s(), 0.14f * s()))
    }
}

/** A kalasha with mango leaves and a coconut, the leaves stirring. */
private fun DrawScope.kalasha(c: Color, t: Float) {
    for (k in -2..2) {
        rotate(k * 22f + 5f * wave(t, 2f, k * 0.15f), p(0.5f, 0.38f)) {
            val leaf = Path().apply {
                moveTo(0.5f * s(), 0.38f * s())
                quadraticBezierTo(0.42f * s(), 0.22f * s(), 0.5f * s(), 0.06f * s())
                quadraticBezierTo(0.58f * s(), 0.22f * s(), 0.5f * s(), 0.38f * s())
            }
            drawPath(leaf, c.copy(alpha = 0.3f)); drawPath(leaf, c, style = thin(c))
        }
    }
    drawCircle(c, 0.12f * s(), p(0.5f, 0.32f))
    drawRect(c, p(0.38f, 0.42f), Size(0.24f * s(), 0.05f * s()))
    val pot = Path().apply {
        moveTo(0.42f * s(), 0.47f * s())
        cubicTo(0.12f * s(), 0.52f * s(), 0.14f * s(), 0.9f * s(), 0.5f * s(), 0.9f * s())
        cubicTo(0.86f * s(), 0.9f * s(), 0.88f * s(), 0.52f * s(), 0.58f * s(), 0.47f * s()); close()
    }
    drawPath(pot, c.copy(alpha = 0.22f)); drawPath(pot, c, style = line(c))
    drawLine(c.copy(alpha = 0.8f), p(0.24f, 0.66f), p(0.76f, 0.66f), s() * 0.03f)
    for (k in 0 until 3) drawCircle(c, 0.022f * s(), p(0.38f + 0.12f * k, 0.74f))
}

/** A birth star, turning slowly and twinkling. */
private fun DrawScope.star(c: Color, t: Float) {
    val tw = 0.88f + 0.12f * wave(t, 4f)
    rotate(t * 90f) {
        val path = Path()
        for (i in 0 until 16) {
            val a = TAU * i / 16 - PI.toFloat() / 2
            val r = (if (i % 2 == 0) 0.4f else 0.16f) * tw * s()
            val x = center.x + cos(a) * r; val y = center.y + sin(a) * r
            if (i == 0) path.moveTo(x, y) else path.lineTo(x, y)
        }
        path.close()
        drawPath(path, c.copy(alpha = 0.35f)); drawPath(path, c, style = thin(c))
    }
    drawCircle(Color.White.copy(alpha = 0.6f), 0.06f * s() * tw, center)
    for ((i, xy) in listOf(0.14f to 0.2f, 0.86f to 0.82f, 0.84f to 0.16f).withIndex()) {
        val on = (0.5f + 0.5f * wave(t, 3f, i * 0.33f)).let { it * it }
        sparkle(c.copy(alpha = on), p(xy.first, xy.second), s() * 0.05f * on)
    }
}

/** A Sri-Yantra-like figure: the triangles turn against each other, the centre breathes. */
private fun DrawScope.yantra(c: Color, t: Float) {
    drawRect(c, p(0.08f, 0.08f), Size(0.84f * s(), 0.84f * s()), style = thin(c))
    drawCircle(c.copy(alpha = 0.8f), 0.36f * s(), center, style = thin(c))
    for ((dir, spin) in listOf(1f to 1f, -1f to -1f)) {
        rotate(spin * t * 120f) {
            val path = Path()
            for (i in 0 until 3) {
                val a = TAU * i / 3 - dir * PI.toFloat() / 2
                val x = center.x + cos(a) * 0.3f * s(); val y = center.y + sin(a) * 0.3f * s()
                if (i == 0) path.moveTo(x, y) else path.lineTo(x, y)
            }
            path.close()
            drawPath(path, c.copy(alpha = 0.12f)); drawPath(path, c, style = thin(c))
        }
    }
    drawCircle(c, s() * (0.04f + 0.015f * wave(t, 4f)), center)
}

/** Planets on their rings around a bright centre. */
private fun DrawScope.orbit(c: Color, t: Float) {
    drawCircle(c.copy(alpha = 0.45f), 0.22f * s(), center, style = thin(c))
    drawCircle(c.copy(alpha = 0.35f), 0.4f * s(), center, style = thin(c))
    drawCircle(c, 0.1f * s(), center)
    val a1 = TAU * t * 3f; val a2 = TAU * t * 1.5f + 2f; val a3 = a2 + PI.toFloat()
    drawCircle(c, 0.055f * s(), Offset(center.x + cos(a1) * 0.22f * s(), center.y + sin(a1) * 0.22f * s()))
    drawCircle(c, 0.065f * s(), Offset(center.x + cos(a2) * 0.4f * s(), center.y + sin(a2) * 0.4f * s()))
    drawCircle(c.copy(alpha = 0.7f), 0.04f * s(), Offset(center.x + cos(a3) * 0.4f * s(), center.y + sin(a3) * 0.4f * s()))
}

/** A conch, sounding: its call spreads in rings. */
private fun DrawScope.conch(c: Color, t: Float) {
    for (i in 0 until 3) {
        val w = frac(t * 3f + i / 3f)
        val r = (0.12f + 0.3f * w) * s()
        drawArc(c.copy(alpha = (1f - w) * 0.8f), -40f, 80f, false, Offset(0.62f * s() - r, 0.42f * s() - r), Size(2 * r, 2 * r), style = thin(c))
    }
    rotate(-10f + 4f * wave(t, 3f), p(0.38f, 0.55f)) {
        val shell = Path().apply {
            moveTo(0.1f * s(), 0.62f * s())
            cubicTo(0.1f * s(), 0.3f * s(), 0.5f * s(), 0.22f * s(), 0.62f * s(), 0.42f * s())
            cubicTo(0.66f * s(), 0.52f * s(), 0.58f * s(), 0.74f * s(), 0.4f * s(), 0.76f * s())
            cubicTo(0.28f * s(), 0.78f * s(), 0.14f * s(), 0.74f * s(), 0.1f * s(), 0.62f * s()); close()
        }
        drawPath(shell, c.copy(alpha = 0.22f)); drawPath(shell, c, style = line(c))
        drawArc(c, 200f, 220f, false, p(0.26f, 0.44f), Size(0.24f * s(), 0.22f * s()), style = thin(c))
        drawArc(c, 200f, 220f, false, p(0.32f, 0.5f), Size(0.12f * s(), 0.1f * s()), style = thin(c))
    }
}

/** A kolam, drawn line by line around its dots, then begun again. */
private fun DrawScope.kolam(c: Color, t: Float) {
    for (i in -1..1) for (j in -1..1) drawCircle(c, 0.03f * s(), p(0.5f + 0.24f * i, 0.5f + 0.24f * j))
    val prog = frac(t * 1.5f)
    val n = 160
    val upto = (prog * 1.25f * n).toInt().coerceAtMost(n)
    val fade = if (prog > 0.8f) 1f - (prog - 0.8f) / 0.2f else 1f
    val path = Path()
    for (k in 0..upto) {
        val th = TAU * k / n
        val x = 0.5f + 0.37f * sin(2 * th + PI.toFloat() / 2) * cos(th)
        val y = 0.5f + 0.37f * sin(3 * th)
        if (k == 0) path.moveTo(x * s(), y * s()) else path.lineTo(x * s(), y * s())
    }
    drawPath(path, c.copy(alpha = fade), style = thin(c))
}

/** The sun, turning. */
private fun DrawScope.sun(c: Color, t: Float) {
    rotate(t * 60f) {
        for (k in 0 until 12) {
            val a = TAU * k / 12
            val long = k % 2 == 0
            val r0 = 0.27f * s(); val r1 = (if (long) 0.45f else 0.38f) * s() * (0.95f + 0.05f * wave(t, 4f, k / 12f))
            drawLine(c, Offset(center.x + cos(a) * r0, center.y + sin(a) * r0), Offset(center.x + cos(a) * r1, center.y + sin(a) * r1), s() * 0.05f, StrokeCap.Round)
        }
    }
    drawCircle(Brush.radialGradient(listOf(Color.White.copy(alpha = 0.7f), c), center, 0.22f * s()), 0.2f * s(), center)
}

/** An arrow that flies to the centre of the target and stays. */
private fun DrawScope.bow(c: Color, t: Float) {
    val o = p(0.66f, 0.5f)
    for ((i, r) in listOf(0.28f, 0.18f, 0.08f).withIndex()) {
        if (i == 2) drawCircle(c, r * s(), o) else drawCircle(c, r * s(), o, style = thin(c))
    }
    val a = frac(t * 2.5f)
    val alpha = if (a > 0.85f) 1f - (a - 0.85f) / 0.15f else 1f
    val fly = ease(a / 0.35f)
    val tipX = 0.04f + (0.62f - 0.04f) * fly
    val wob = if (a > 0.35f) 4f * sin(TAU * (a - 0.35f) * 6f) * (1f - ((a - 0.35f) / 0.5f).coerceIn(0f, 1f)) else 0f
    rotate(wob, p(tipX, 0.5f)) {
        val cc = c.copy(alpha = alpha)
        drawLine(cc, p(tipX - 0.5f, 0.5f), p(tipX, 0.5f), s() * 0.04f, StrokeCap.Round)
        drawPath(Path().apply { moveTo(tipX * s(), 0.5f * s()); lineTo((tipX - 0.1f) * s(), 0.44f * s()); lineTo((tipX - 0.1f) * s(), 0.56f * s()); close() }, cc)
        drawLine(cc, p(tipX - 0.46f, 0.5f), p(tipX - 0.52f, 0.42f), s() * 0.03f, StrokeCap.Round)
        drawLine(cc, p(tipX - 0.46f, 0.5f), p(tipX - 0.52f, 0.58f), s() * 0.03f, StrokeCap.Round)
    }
}
