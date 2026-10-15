package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AreaReading
import uniffi.kshana_ffi.TimingRecord

private val LEVEL_LABELS = listOf("Maha", "Antar", "Pratyantar", "Sookshma", "Prana")

/** Life areas: pick one, read its promise and the five-level timing funnel. */
@Composable
fun AreasScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    var readings by remember { mutableStateOf<List<AreaReading>>(emptyList()) }
    var selected by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var status by remember { mutableStateOf("judging eight areas…") }
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()

    LaunchedEffect(profile, settings) {
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            readings = withContext(Dispatchers.Default) {
                engine.areaReadings(birth, profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, Instants.now(), 5u)
            }
            if (selected == null) selected = readings.firstOrNull()?.promise?.area
            status = "${readings.size} areas · judged at ${localText(Instants.now(), DATE_TIME_S)}"
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("LIFE AREAS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
            }
            Text("Where the period leans", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Text(status, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            ErrorLine(error)
        }

        // Chip row with a fade and a gold "more" arrow at the right edge.
        Box(Modifier.fillMaxWidth().height(44.dp)) {
            LazyRow(state = listState, contentPadding = PaddingValues(start = 24.dp, end = 80.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(readings, key = { it.promise.area }) { r ->
                    Chip(r.promise.area.uppercase(), selected = r.promise.area == selected, onClick = { selected = r.promise.area })
                }
            }
            Box(
                Modifier.align(Alignment.CenterEnd).width(96.dp).fillMaxHeight()
                    .background(Brush.horizontalGradient(listOf(Color.Transparent, KshanaColors.Night, KshanaColors.Night))),
            )
            Box(
                Modifier.align(Alignment.CenterEnd).padding(end = 24.dp).size(44.dp).clip(CircleShape).background(KshanaColors.Gold)
                    .clickable {
                        scope.launch {
                            val next = (listState.firstVisibleItemIndex + 2).coerceAtMost((readings.size - 1).coerceAtLeast(0))
                            listState.animateScrollToItem(next)
                        }
                    },
                contentAlignment = Alignment.Center,
            ) {
                Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = "more areas", tint = KshanaColors.Night)
            }
        }

        val r = readings.firstOrNull { it.promise.area == selected }
        if (r != null) {
            Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                val p = r.promise
                SectionLabel("Promise")
                Text(
                    if (p.promised) "${p.area} is promised · strength ${"%.2f".format(p.strength)}" else "${p.area} is not promised · strength ${"%.2f".format(p.strength)}",
                    fontFamily = FontFamily.Serif, fontSize = 20.sp, color = if (p.promised) KshanaColors.Gold else KshanaColors.TextSoft,
                )
                Text(
                    "cusp ${p.primaryCusp} · group ${p.group.joinToString(" ")} · sub lord ${p.subLord} · ${p.ruleId} · ${p.source}",
                    fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted,
                )
                for (f in p.factors) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Text(f, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.padding(vertical = 6.dp))
                }
                Text(
                    "Parashari ${"%.2f".format(r.parashariScore)} · lord ${r.parashariLord} · ${if (r.traditionsAgree) "traditions agree" else "traditions disagree"}",
                    style = MaterialTheme.typography.bodySmall, color = if (r.traditionsAgree) KshanaColors.Gold else KshanaColors.Sky,
                )

                SectionLabel("Timing funnel")
                Text("${r.agreeingLevels} of 5 levels support it", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                val levels = r.funnel.sortedBy { it.level.toInt() }
                for (i in 0 until 5) {
                    val t = levels.getOrNull(i)
                    FunnelRow(LEVEL_LABELS[i], t, widthFraction = 1f - i * 0.135f, dim = i >= r.agreeingLevels.toInt())
                }
                val tightest = levels.lastOrNull { it.supports.isNotEmpty() } ?: levels.getOrNull(r.agreeingLevels.toInt() - 1)
                Spacer(Modifier.height(6.dp))
                if (tightest != null) {
                    Text("TIGHTEST SLOT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text("${LEVEL_LABELS.getOrElse(tightest.level.toInt() - 1) { "level ${tightest.level}" }} · ${tightest.lord}", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
                    Text("${localText(tightest.start, DATE_TIME_S)} → ${localText(tightest.end, DATE_TIME_S)}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
                    Text("${microText(tightest.start)} → ${microText(tightest.end)}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Gold)
                } else {
                    Text("No level of the funnel supports this area now.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                }
            }
        }
    }
}

@Composable
private fun FunnelRow(label: String, t: TimingRecord?, widthFraction: Float, dim: Boolean) {
    val alpha = if (dim) 0.35f else 1f
    Column(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted.copy(alpha = alpha), modifier = Modifier.width(96.dp))
            Text(t?.lord ?: "—", fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text.copy(alpha = alpha), modifier = Modifier.weight(1f))
            if (t != null) {
                Text(
                    "${localText(t.start, DATE_TIME)} → ${localText(t.end, DATE_TIME)}",
                    fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Sky.copy(alpha = alpha),
                )
            }
        }
        Box(
            Modifier.fillMaxWidth(widthFraction).height(6.dp).clip(RoundedCornerShape(3.dp))
                .background((if (t != null && t.supports.isNotEmpty()) KshanaColors.Gold else KshanaColors.Line).copy(alpha = alpha)),
        )
    }
}
