package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.Gazetteer
import com.kshana.app.data.Place
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.sky.KshanaFonts
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext

/**
 * Search every town and village, offline: type "Aluva" and pick "Aluva · Ernākulam,
 * Kerala", so the reading uses the town's own coordinates, not the nearest big city's.
 */
@Composable
fun PlacePicker(label: String = "Search your town or village", onPick: (Place) -> Unit) {
    val context = LocalContext.current
    var query by remember { mutableStateOf("") }
    var hits by remember { mutableStateOf<List<Gazetteer.Hit>>(emptyList()) }
    var busy by remember { mutableStateOf(false) }
    LaunchedEffect(query) {
        if (Gazetteer.fold(query).trim().length < 2) { hits = emptyList(); return@LaunchedEffect }
        delay(250)
        busy = true
        hits = runCatching { withContext(Dispatchers.IO) { Gazetteer.search(context, query) } }.getOrDefault(emptyList())
        busy = false
    }
    Column(Modifier.fillMaxWidth()) {
        Field(query, { query = it }, label, Modifier.fillMaxWidth())
        if (busy && hits.isEmpty()) Text("searching every town and village…", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 6.dp))
        if (!busy && hits.isEmpty() && Gazetteer.fold(query).trim().length >= 2) {
            Text("No place starts with that name. Try another spelling (Alwaye for Aluva, Calicut for Kozhikode), or search online below.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 6.dp))
        }
        for (h in hits) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Row(
                Modifier.fillMaxWidth().heightIn(min = 48.dp).clickable {
                    onPick(h.place())
                    query = ""
                }.padding(vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(h.name, fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text)
                    if (h.region.isNotBlank()) Text(h.region, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
                Text("%.4f° %.4f°".format(h.latitude, h.longitude), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
            }
        }
        if (hits.isNotEmpty()) HorizontalDivider(color = KshanaColors.LineSoft)
    }
}
