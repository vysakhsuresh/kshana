package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import com.kshana.app.ui.theme.Grotesk
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.halo
import uniffi.kshana_ffi.ExactInstant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

/* Shared, package-private helpers for the screens. `signName` and `dms` live in NowScreen.kt. */

internal val PLANET_GLYPHS: Map<String, String> = mapOf(
    "Sun" to "☉", "Moon" to "☽", "Mercury" to "☿", "Venus" to "♀", "Mars" to "♂",
    "Jupiter" to "♃", "Saturn" to "♄", "Uranus" to "♅", "Neptune" to "♆", "Pluto" to "♇",
    "Rahu" to "☊", "Ketu" to "☋",
    // The engine names the planets in Malayalam and Hindi on those languages' chart pages.
    "സൂര്യൻ" to "☉", "ചന്ദ്രൻ" to "☽", "ബുധൻ" to "☿", "ശുക്രൻ" to "♀", "ചൊവ്വ" to "♂",
    "വ്യാഴം" to "♃", "ശനി" to "♄", "യുറാനസ്" to "♅", "നെപ്റ്റ്യൂൺ" to "♆", "പ്ലൂട്ടോ" to "♇",
    "രാഹു" to "☊", "കേതു" to "☋",
    "सूर्य" to "☉", "चंद्र" to "☽", "बुध" to "☿", "शुक्र" to "♀", "मंगल" to "♂",
    "गुरु" to "♃", "शनि" to "♄", "अरुण" to "♅", "वरुण" to "♆", "यम" to "♇",
    "राहु" to "☊", "केतु" to "☋",
)

internal val PLANET_ABBR: Map<String, String> = mapOf(
    "Sun" to "Su", "Moon" to "Mo", "Mars" to "Ma", "Mercury" to "Me", "Jupiter" to "Ju",
    "Venus" to "Ve", "Saturn" to "Sa", "Rahu" to "Ra", "Ketu" to "Ke",
    "Uranus" to "Ur", "Neptune" to "Ne", "Pluto" to "Pl",
    // The short forms Kerala and North Indian charts print in their boxes.
    "സൂര്യൻ" to "ര", "ചന്ദ്രൻ" to "ച", "ചൊവ്വ" to "കു", "ബുധൻ" to "ബു", "വ്യാഴം" to "ഗു",
    "ശുക്രൻ" to "ശു", "ശനി" to "മ", "രാഹു" to "സ", "കേതു" to "ശി",
    "सूर्य" to "सू", "चंद्र" to "चं", "मंगल" to "मं", "बुध" to "बु", "गुरु" to "गु",
    "शुक्र" to "शु", "शनि" to "श", "राहु" to "रा", "केतु" to "के",
)

internal val SIGN_SHORT = listOf("Ari", "Tau", "Gem", "Can", "Leo", "Vir", "Lib", "Sco", "Sag", "Cap", "Aqu", "Pis")

internal val DATE_TIME = DateTimeFormatter.ofPattern("d MMM uuuu HH:mm")
internal val DATE_TIME_S = DateTimeFormatter.ofPattern("d MMM uuuu HH:mm:ss")
internal val DATE_ONLY = DateTimeFormatter.ofPattern("d MMM uuuu")
internal val TIME_ONLY = DateTimeFormatter.ofPattern("HH:mm:ss")

internal fun norm360(x: Double): Double = (x % 360.0 + 360.0) % 360.0

/** Degrees and minutes within the sign, e.g. 17°42′. */
internal fun degMin(lon: Double): String {
    val inSign = norm360(lon) % 30.0
    val d = inSign.toInt()
    val m = ((inSign - d) * 60.0).toInt()
    return "%d°%02d′".format(d, m)
}

internal fun signIndex(lon: Double): Int = (norm360(lon) / 30.0).toInt() % 12

/** Seconds since the TAI epoch as a double, for progress bars and ordering. */
internal fun ExactInstant.seconds(): Double = taiSeconds.toDouble() + nanos.toDouble() / 1e9

/** ".nnn nnn" microseconds within the second. */
internal fun microText(e: ExactInstant): String {
    val micro = (e.nanos.toLong() / 1_000).toString().padStart(6, '0')
    return ".${micro.substring(0, 3)} ${micro.substring(3)}"
}

internal fun localText(e: ExactInstant, pattern: DateTimeFormatter = DATE_TIME, zone: ZoneId = ZoneId.systemDefault()): String =
    Instants.toLocal(e, zone).format(pattern)

/** Fraction of the way from start to end, clamped. */
internal fun progress(start: ExactInstant, end: ExactInstant, at: ExactInstant): Float {
    val span = end.seconds() - start.seconds()
    if (span <= 0.0) return 0f
    return ((at.seconds() - start.seconds()) / span).coerceIn(0.0, 1.0).toFloat()
}

/** A factor string as a sentence: capitalised, ending in a full stop. Nothing is added. */
internal fun sentence(factor: String): String {
    val t = factor.trim()
    if (t.isEmpty()) return t
    val s = t.replaceFirstChar { it.uppercaseChar() }
    return if (s.endsWith(".") || s.endsWith("!") || s.endsWith("?")) s else "$s."
}

/** A section title: small capitals, a diamond, and a gold hairline that fades out. */
@Composable
internal fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Row(modifier.padding(top = 18.dp, bottom = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("◆", fontSize = 7.sp, color = KshanaColors.Gold, modifier = Modifier.padding(end = 8.dp))
        Text(text.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Box(
            Modifier.padding(start = 10.dp).height(1.dp).width(72.dp)
                .background(androidx.compose.ui.graphics.Brush.horizontalGradient(listOf(KshanaColors.Gold.copy(alpha = 0.45f), androidx.compose.ui.graphics.Color.Transparent))),
        )
    }
}

/** The engine at work: the wheel of twenty-seven points and a few words. */
@Composable
internal fun Working(text: String, modifier: Modifier = Modifier) {
    Row(modifier.padding(vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        com.kshana.app.ui.theme.StarWheel(size = 36.dp, modifier = Modifier.size(36.dp))
        Spacer(Modifier.width(12.dp))
        Text(text, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    }
}

@Composable
internal fun ErrorLine(error: String?) {
    if (error != null) {
        Text(error, color = KshanaColors.Sky, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(vertical = 6.dp))
    }
}

/** A 44 dp pill chip in the night palette: the one filled selection on a screen (a topic). */
@Composable
internal fun Chip(label: String, selected: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(22.dp)
    Box(
        modifier = modifier
            .height(44.dp)
            .clip(shape)
            .background(if (selected) KshanaColors.Gold else KshanaColors.Surface)
            .border(1.dp, if (selected) KshanaColors.Gold else KshanaColors.Line, shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            label,
            fontFamily = Grotesk,
            fontWeight = FontWeight.Medium,
            fontSize = 12.sp,
            letterSpacing = 1.sp,
            color = if (selected) KshanaColors.Night else KshanaColors.TextSoft,
        )
    }
}

/**
 * A quiet pill for a value the person can change (a date, a time): outlined, never
 * filled, so the gold fill stays for the topic chip and the one call to action.
 */
@Composable
internal fun ValuePill(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(22.dp)
    Box(
        modifier = modifier
            .height(44.dp)
            .clip(shape)
            .background(KshanaColors.Surface)
            .border(1.dp, KshanaColors.Gold.copy(alpha = 0.7f), shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = KshanaColors.Gold)
    }
}

/**
 * One option among several (a question, a purpose): a surface box with a small ring on
 * the left. Selected: gold ring filled, gold border, text in the UI sans. Never filled,
 * so a screen keeps one gold block, the button.
 */
@Composable
internal fun OptionBox(label: String, selected: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(12.dp)
    Row(
        modifier = modifier
            .clip(shape)
            .background(KshanaColors.Surface)
            .border(1.dp, if (selected) KshanaColors.Gold else KshanaColors.LineSoft, shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier
                .size(10.dp)
                .clip(CircleShape)
                .border(1.5.dp, if (selected) KshanaColors.Gold else KshanaColors.Muted, CircleShape)
                .background(if (selected) KshanaColors.Gold else Color.Transparent),
        )
        Spacer(Modifier.width(10.dp))
        Text(
            label,
            fontFamily = Grotesk,
            fontWeight = if (selected) FontWeight.Medium else FontWeight.Normal,
            fontSize = 14.sp,
            lineHeight = 18.sp,
            color = if (selected) KshanaColors.Gold else KshanaColors.Text,
        )
    }
}

/** The one gold call-to-action: full width, 52 dp, serif capitals, a soft light behind it. */
@Composable
internal fun GoldButton(label: String, enabled: Boolean = true, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(10.dp)
    Box(
        modifier = modifier
            .height(52.dp)
            .then(if (enabled) Modifier.com_kshana_halo() else Modifier)
            .clip(shape)
            .background(
                if (enabled) androidx.compose.ui.graphics.Brush.verticalGradient(listOf(Color(0xFFEBC77E), KshanaColors.Gold, Color(0xFFCFA557)))
                else androidx.compose.ui.graphics.Brush.verticalGradient(listOf(KshanaColors.Line, KshanaColors.Line)),
            )
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, fontFamily = KshanaFonts.serif, fontSize = 16.sp, letterSpacing = 4.sp, color = if (enabled) KshanaColors.Night else KshanaColors.Muted)
    }
}

private fun Modifier.com_kshana_halo(): Modifier = this.halo(KshanaColors.Gold, radiusFactor = 0.9f, alpha = 0.16f)

/** A quiet secondary action: a gold hairline, 44 dp. */
@Composable
internal fun QuietButton(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(10.dp)
    Box(
        modifier = modifier
            .height(44.dp)
            .clip(shape)
            .background(KshanaColors.Surface.copy(alpha = 0.5f))
            .border(1.dp, KshanaColors.Gold.copy(alpha = 0.38f), shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
    }
}

/** A label/value pair in a row: label muted sans, value mono sky. */
@Composable
internal fun MonoRow(label: String, value: String, valueColor: androidx.compose.ui.graphics.Color = KshanaColors.Sky) {
    Row(Modifier.padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
        Text(value, fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = valueColor)
    }
}

/**
 * Shown when someone keeps returning to a painful question. The app reads seasons; it
 * does not carry a person through them. It says so, gently, and points to people who can.
 */
@Composable
internal fun CareCard() {
    val context = androidx.compose.ui.platform.LocalContext.current
    androidx.compose.foundation.layout.Column(
        Modifier.padding(vertical = 6.dp).then(Modifier).let { it }
            .clip(RoundedCornerShape(16.dp))
            .background(KshanaColors.Sky.copy(alpha = 0.08f))
            .border(1.dp, KshanaColors.Sky.copy(alpha = 0.5f), RoundedCornerShape(16.dp))
            .padding(16.dp),
        verticalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(8.dp),
    ) {
        Text("YOU ARE NOT ALONE IN THIS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Sky)
        Text(
            "You have come back to this question a few times. A hard season passes, and the dates above show when it eases. If it feels too heavy to carry today, talk to someone who listens: a friend, family, or a counsellor.",
            fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 23.sp, color = KshanaColors.Text,
        )
        Text("Tele-MANAS, the Government of India's free helpline, answers day and night, in your language: 14416.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        QuietButton("CALL 14416", onClick = {
            runCatching { context.startActivity(android.content.Intent(android.content.Intent.ACTION_DIAL, android.net.Uri.parse("tel:14416"))) }
        })
    }
}

/**
 * An open door: wherever the app gives a short answer, one of these leads to the whole
 * of it. A gold label, an arrow, and a hairline above, so it reads as a way further in.
 */
@Composable
internal fun Door(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    androidx.compose.foundation.layout.Column(modifier.fillMaxWidth()) {
        Box(Modifier.fillMaxWidth().height(1.dp).background(KshanaColors.Gold.copy(alpha = 0.18f)))
        Row(
            Modifier.fillMaxWidth().clickable(onClick = onClick).padding(vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
            Text("›", fontSize = 18.sp, color = KshanaColors.Gold)
        }
    }
}
