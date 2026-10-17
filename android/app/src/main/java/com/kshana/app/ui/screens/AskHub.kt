package com.kshana.app.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place

/**
 * Ask: two doors on one tab. A question about a life or a matter; or a time you have,
 * read for what it is for. The person picks which; nothing else changes.
 */
@Composable
fun AskHub(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    initialMode: String = "question",
    onRemedy: (String) -> Unit,
    onLife: (String) -> Unit,
) {
    var mode by rememberSaveable { mutableStateOf(initialMode) }
    Box(Modifier.fillMaxSize()) {
        when (mode) {
            "time" -> PlanScreen(profile = profile, place = place, settings = settings, onLife = onLife)
            else -> AskScreen(profile = profile, place = place, settings = settings, onRemedy = onRemedy)
        }
        // The mode switch floats over the top of either screen.
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 24.dp).padding(top = 8.dp),
            horizontalArrangement = Arrangement.End,
        ) {
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Chip("A QUESTION", selected = mode == "question", onClick = { mode = "question" })
                Chip("A TIME", selected = mode == "time", onClick = { mode = "time" })
            }
        }
    }
}
