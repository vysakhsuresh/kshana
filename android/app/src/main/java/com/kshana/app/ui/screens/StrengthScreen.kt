package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.engine.EngineProvider
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AvasthaRecord
import uniffi.kshana_ffi.BhavaBalaRecord
import uniffi.kshana_ffi.GrahaBalaRecord
import uniffi.kshana_ffi.ShadbalaRecord
import kotlin.math.max
import kotlin.math.roundToInt
import com.kshana.app.ui.sky.KshanaFonts

private val CHART_HEIGHT = 240.dp

/** One-line meanings for the states, BPHS ch. 45. */
private val BALADI_WORDS = mapOf(
    "Bala" to "infant, a quarter of its fruit",
    "Kumara" to "youth, half its fruit",
    "Yuva" to "adult, its full fruit",
    "Vriddha" to "old, a trace of its fruit",
    "Mrita" to "dead, no fruit",
)
private val JAGRADADI_WORDS = mapOf(
    "Jagrat" to "awake, full results",
    "Swapna" to "dreaming, middling results",
    "Sushupti" to "asleep, no results",
)
private val DEEPTADI_WORDS = mapOf(
    "Deepta" to "bright, exalted",
    "Swastha" to "at ease, in its own sign",
    "Mudita" to "glad, in a friend's sign",
    "Shanta" to "calm, in a neutral's sign",
    "Deena" to "poor, in an enemy's sign",
    "Dukhita" to "grieving, debilitated",
    "Vikala" to "maimed, joined by a malefic",
    "Khala" to "wicked, in a great enemy's sign",
    "Kopa" to "angry, combust",
)
/** The eight motional states of the five planets, BPHS 27. */
private val MOTION_WORDS = mapOf(
    "Vakra" to "retrograde",
    "Anuvakra" to "retrograde into the previous sign",
    "Vikala" to "stationary",
    "Manda" to "slow",
    "Mandatara" to "very slow",
    "Sama" to "even",
    "Chara" to "fast",
    "Atichara" to "very fast",
)

private fun v1(x: Double): String = "%.1f".format(x)
private fun pct(ratio: Double): String = "${(ratio * 100.0).roundToInt()}%"
private fun strengthColour(ratio: Double): Color = if (ratio >= 1.0) KshanaColors.Gold else KshanaColors.Sky

/**
 * "How strong each planet stands": the Shadbala of the seven grahas as bars against the
 * required line, the six parts of the chosen graha, the time lords at birth, the planetary
 * wars, the twelve bhava balas and the avasthas.
 */
@Composable
fun StrengthScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    var record by remember(profile, settings) { mutableStateOf<ShadbalaRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var selected by remember(profile, settings) { mutableStateOf<Int?>(null) }

    LaunchedEffect(profile, settings) {
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            record = withContext(Dispatchers.Default) {
                engine.shadbala(birth, profile.latitude, profile.longitude, settings.ayanamsa)
            }
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("SHADBALA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            QuietButton("BACK", onClick = onBack)
        }
        Text("How strong each planet stands", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "The six-fold strength of the seven grahas in your chart · ${profile.placeName}",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        ErrorLine(error)

        val rec = record
        if (rec == null) {
            if (error == null) Text("weighing the seven grahas…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            SectionLabel("Strength against the required")
            StrengthChart(rec.grahas, selected) { i -> selected = if (selected == i) null else i }
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                Legend(KshanaColors.Gold, "enough")
                Legend(KshanaColors.Sky, "short")
                Text("— required", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            }
            Text(
                "Your ${rec.strongest} stands strongest; your ${rec.weakest} weakest.",
                fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 24.sp, color = KshanaColors.Text,
            )
            val chosen = selected?.let { rec.grahas.getOrNull(it) }
            if (chosen == null) {
                Text("Tap a bar to open its six parts.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            } else {
                GrahaCard(chosen)
            }

            SectionLabel("Time lords at birth")
            MonoRow("Year lord", rec.yearLord)
            MonoRow("Month lord", rec.monthLord)
            MonoRow("Day lord", rec.dayLord)
            MonoRow("Hora lord", rec.horaLord)

            if (rec.wars.isNotEmpty()) {
                SectionLabel("Planetary wars")
                for (w in rec.wars) {
                    Text(sentence(w), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                }
            }

            SectionLabel("Houses")
            val maxRatio = rec.bhavas.maxOfOrNull { it.ratio }?.takeIf { it > 0.0 } ?: 1.0
            for (b in rec.bhavas) HouseRow(b, maxRatio)

            SectionLabel("Avasthas")
            Row(Modifier.fillMaxWidth()) {
                Text("GRAHA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1.1f))
                Text("BALADI", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
                Text("JAGRADADI", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
                Text("DEEPTADI", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
            }
            for (a in rec.avasthas) AvasthaRow(a)
        }

        Text(
            "BPHS chapters 27 and 45; conventions stated in the engine's docs.",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
            modifier = Modifier.padding(vertical = 8.dp),
        )
    }
}

/* ---------------------------------------------------------------------------------- */

/**
 * Seven bars of total virupas, the required minimum as a dashed line across each, the
 * glyph beneath and the ratio above; gold when the graha has enough, sky when short.
 * A tap on a slot chooses that graha.
 */
@Composable
private fun StrengthChart(grahas: List<GrahaBalaRecord>, selected: Int?, onSelect: (Int) -> Unit) {
    val measurer = rememberTextMeasurer()
    val glyphStyle = remember { TextStyle(fontFamily = KshanaFonts.mono, fontSize = 15.sp) }
    val pctStyle = remember { TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp) }
    val glyphs = remember(grahas, measurer) { grahas.map { measurer.measure(PLANET_GLYPHS[it.graha] ?: it.graha.take(2), glyphStyle) } }
    val pcts = remember(grahas, measurer) { grahas.map { measurer.measure(pct(it.ratio), pctStyle) } }
    val maxV = remember(grahas) { grahas.maxOfOrNull { max(it.totalVirupas, it.required) }?.takeIf { it > 0.0 } ?: 1.0 }
    val n = grahas.size

    Canvas(
        Modifier.fillMaxWidth().height(CHART_HEIGHT).pointerInput(n) {
            detectTapGestures { off ->
                if (n == 0) return@detectTapGestures
                val slot = size.width.toFloat() / n.toFloat()
                val i = (off.x / slot).toInt()
                if (i in 0 until n) onSelect(i)
            }
        },
    ) {
        if (n == 0) return@Canvas
        val w = size.width
        val h = size.height
        val top = 18.dp.toPx()
        val bottom = h - 26.dp.toPx()
        val slot = w / n.toFloat()
        val barW = slot * 0.46f
        val hair = 1.dp.toPx()
        val dash = PathEffect.dashPathEffect(floatArrayOf(5.dp.toPx(), 3.dp.toPx()), 0f)
        val span = bottom - top
        fun y(v: Double): Float = bottom - (v / maxV).toFloat() * span

        // The floor.
        drawLine(KshanaColors.Line, Offset(0f, bottom), Offset(w, bottom), strokeWidth = hair)

        for (i in 0 until n) {
            val g = grahas[i]
            val cx = slot * i.toFloat() + slot / 2f
            val x0 = cx - barW / 2f
            val yTop = y(g.totalVirupas)
            val colour = strengthColour(g.ratio)
            val alpha = if (selected == null || selected == i) 1f else 0.45f
            drawRect(colour, Offset(x0, yTop), Size(barW, bottom - yTop), alpha = alpha)
            if (selected == i) {
                drawRect(KshanaColors.Text, Offset(x0 - hair, yTop - hair), Size(barW + 2f * hair, bottom - yTop + hair), style = Stroke(hair), alpha = 0.8f)
            }
            // The required minimum across the slot.
            val yReq = y(g.required)
            val inset = slot * 0.12f
            drawLine(KshanaColors.Text, Offset(slot * i.toFloat() + inset, yReq), Offset(slot * (i + 1).toFloat() - inset, yReq), strokeWidth = hair, alpha = 0.75f, pathEffect = dash)
            // The ratio above the bar, the glyph beneath the floor.
            val p = pcts[i]
            drawText(p, color = if (selected == i) KshanaColors.Text else KshanaColors.Muted, topLeft = Offset(cx - p.size.width / 2f, yTop - p.size.height - 2.dp.toPx()))
            val gl = glyphs[i]
            drawText(gl, color = if (selected == i) KshanaColors.Gold else KshanaColors.TextSoft, topLeft = Offset(cx - gl.size.width / 2f, bottom + 4.dp.toPx()))
        }
    }
}

@Composable
private fun Legend(colour: Color, label: String) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        Box(Modifier.width(10.dp).height(10.dp).clip(RoundedCornerShape(2.dp)).background(colour))
        Text(label, fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
    }
}

/* ---------------------------------------------------------------------------------- */

/** The chosen graha: its six parts, the sub-parts on request, the motion, Ishta and Kashta, the words. */
@Composable
private fun GrahaCard(g: GrahaBalaRecord) {
    var detail by remember(g.graha) { mutableStateOf(false) }
    val shape = RoundedCornerShape(8.dp)
    val colour = strengthColour(g.ratio)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(PLANET_GLYPHS[g.graha] ?: g.graha.take(2), fontSize = 24.sp, color = colour)
            Text(g.graha, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
        }
        Text(
            "${v1(g.totalVirupas)} virupas = ${"%.2f".format(g.rupas)} rupas · required ${v1(g.required)} · ${pct(g.ratio)}",
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, lineHeight = 16.sp, color = colour,
        )
        HorizontalDivider(color = KshanaColors.LineSoft)
        MonoRow("Sthana", v1(g.sthana))
        if (detail) {
            SubRow("Uchcha", g.uchcha)
            SubRow("Saptavargaja", g.saptavargaja)
            SubRow("Ojhayugma", g.ojhayugma)
            SubRow("Kendradi", g.kendradi)
            SubRow("Drekkana", g.drekkana)
        }
        MonoRow("Dig", v1(g.dig))
        MonoRow("Kala", v1(g.kala))
        if (detail) {
            SubRow("Nathonnata", g.nathonnata)
            SubRow("Paksha", g.paksha)
            SubRow("Tribhaga", g.tribhaga)
            SubRow("Abda", g.abda)
            SubRow("Masa", g.masa)
            SubRow("Vara", g.vara)
            SubRow("Hora", g.hora)
            SubRow("Ayana", g.ayana)
            SubRow("Yuddha", g.yuddha)
        }
        MonoRow("Chesta", v1(g.chesta))
        MonoRow("Naisargika", v1(g.naisargika))
        MonoRow("Drik", v1(g.drik))
        Text(
            if (detail) "HIDE DETAIL" else "DETAIL",
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { detail = !detail }.padding(vertical = 6.dp),
        )
        HorizontalDivider(color = KshanaColors.LineSoft)
        val motion = g.motion
        MonoRow("Motion", if (motion == null) "— (Ayana and Paksha instead)" else "$motion · ${MOTION_WORDS[motion] ?: motion.lowercase()}", KshanaColors.TextSoft)
        MonoRow("Ishta · Kashta", "${v1(g.ishta)} · ${v1(g.kashta)}")
        if (g.words.isNotEmpty()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            for (wd in g.words) {
                Row(verticalAlignment = Alignment.Top) {
                    Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
                    Text(sentence(wd), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                }
            }
        }
    }
}

/** A sub-part, indented, in a quieter mono. */
@Composable
private fun SubRow(label: String, value: Double) {
    Row(Modifier.padding(start = 16.dp, top = 1.dp, bottom = 1.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
        Text(v1(value), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Muted)
    }
}

/* ---------------------------------------------------------------------------------- */

/** One house: number, lord, total rupas, and a bar of its ratio against the strongest house. */
@Composable
private fun HouseRow(b: BhavaBalaRecord, maxRatio: Double) {
    val colour = strengthColour(b.ratio)
    Column(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(b.house.toInt().toString(), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Muted, modifier = Modifier.width(24.dp))
            Text(
                "${PLANET_GLYPHS[b.lord] ?: ""} ${b.lord}",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f),
            )
            Text("${"%.2f".format(b.rupas)} rupas · ${pct(b.ratio)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = colour)
        }
        Box(Modifier.fillMaxWidth().padding(start = 24.dp).height(3.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.LineSoft)) {
            val f = (b.ratio / maxRatio).toFloat().coerceIn(0f, 1f)
            if (f > 0f) Box(Modifier.fillMaxWidth(f).height(3.dp).background(colour))
        }
    }
}

/** One graha's three states, with their one-line meanings beneath. */
@Composable
private fun AvasthaRow(a: AvasthaRecord) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                "${PLANET_GLYPHS[a.graha] ?: ""} ${a.graha}",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1.1f),
            )
            Text(a.baladi, fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.weight(1f))
            Text(a.jagradadi, fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.weight(1f))
            Text(a.deeptadi, fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.weight(1f))
        }
        Text(
            listOf(
                BALADI_WORDS[a.baladi] ?: a.baladi.lowercase(),
                JAGRADADI_WORDS[a.jagradadi] ?: a.jagradadi.lowercase(),
                DEEPTADI_WORDS[a.deeptadi] ?: a.deeptadi.lowercase(),
            ).joinToString(" · "),
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
    }
}
