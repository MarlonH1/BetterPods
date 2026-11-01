/*
    LibrePods - AirPods liberated from Apple’s ecosystem
    Copyright (C) 2025 LibrePods contributors

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

package me.kavishdevar.librepods.presentation.screens

import androidx.annotation.DrawableRes
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.bluetooth.AACPManager.Companion.ControlCommandIdentifiers
import me.kavishdevar.librepods.data.AirPodsMax
import me.kavishdevar.librepods.data.Capability
import me.kavishdevar.librepods.data.StemAction
import me.kavishdevar.librepods.presentation.components.ChoiceChip
import me.kavishdevar.librepods.presentation.components.HintText
import me.kavishdevar.librepods.presentation.components.SectionHeader
import me.kavishdevar.librepods.presentation.components.SettingsGroup
import me.kavishdevar.librepods.presentation.components.SettingsRow
import me.kavishdevar.librepods.presentation.components.SwitchRow
import me.kavishdevar.librepods.presentation.components.cardColor
import me.kavishdevar.librepods.presentation.viewmodel.AirPodsUiState
import me.kavishdevar.librepods.presentation.viewmodel.AirPodsViewModel

private fun AirPodsUiState.flag(id: ControlCommandIdentifiers) = controlStates[id]?.getOrNull(0) == 0x01.toByte()

@Composable
private fun SubPage(topPadding: Dp, content: androidx.compose.foundation.lazy.LazyListScope.() -> Unit) {
    val bottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding() + 24.dp
    LazyColumn(
        Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.surfaceContainer)
            .padding(horizontal = 16.dp)
    ) {
        item { Spacer(Modifier.height(topPadding)) }
        content()
        item { Spacer(Modifier.height(bottom)) }
    }
}

@Composable
fun SoundScreen(viewModel: AirPodsViewModel, topPadding: Dp) {
    val state by viewModel.uiState.collectAsState()
    val caps = state.capabilities
    val isMax = state.instance?.model is AirPodsMax

    SubPage(topPadding) {
        if (Capability.ADAPTIVE_AUDIO in caps) {
            item { AdaptiveAudioCard(state, viewModel) }
        }

        val pv = Capability.ADAPTIVE_VOLUME in caps
        val ca = Capability.CONVERSATION_AWARENESS in caps
        if (pv || ca) {
            item { SectionHeader(stringResource(R.string.ui_volume_section)) }
            item {
                SettingsGroup(
                    rows = arrayOf(
                        if (pv) { m -> SwitchRow(m, stringResource(R.string.personalized_volume), stringResource(R.string.ui_pv_desc),
                            checked = state.flag(ControlCommandIdentifiers.ADAPTIVE_VOLUME_CONFIG),
                            onCheckedChange = { viewModel.setControlCommandBoolean(ControlCommandIdentifiers.ADAPTIVE_VOLUME_CONFIG, it) }) } else null,
                        if (ca) { m -> SwitchRow(m, stringResource(R.string.conversational_awareness), stringResource(R.string.ui_ca_desc),
                            checked = state.flag(ControlCommandIdentifiers.CONVERSATION_DETECT_CONFIG),
                            onCheckedChange = { viewModel.setControlCommandBoolean(ControlCommandIdentifiers.CONVERSATION_DETECT_CONFIG, it) }) } else null,
                    )
                )
            }
        }

        item { SectionHeader(stringResource(R.string.equalizer)) }
        item {
            SettingsGroup(rows = arrayOf({ m -> SettingsRow(m, stringResource(R.string.ui_custom_eq), onClick = {}, wip = true) }))
        }

        item { SectionHeader(stringResource(R.string.ui_transparency_section)) }
        item {
            SettingsGroup(
                rows = arrayOf(
                    { m -> SwitchRow(m, stringResource(R.string.ui_show_off_mode), stringResource(R.string.ui_show_off_mode_desc),
                        checked = state.offListeningMode, onCheckedChange = viewModel::setOffListeningMode) },
                    if (!isMax) { m -> SettingsRow(m, stringResource(R.string.customize_transparency_mode),
                        stringResource(R.string.ui_transparency_custom_desc), onClick = {}, wip = true) } else null,
                )
            )
        }
        item { HintText(stringResource(R.string.ui_wip_hint)) }
    }
}

@Composable
private fun AdaptiveAudioCard(state: AirPodsUiState, viewModel: AirPodsViewModel) {
    // AUTO_ANC_STRENGTH is inverted on the wire: 0 = let most noise through
    var value by remember {
        mutableFloatStateOf(100f - (state.controlStates[ControlCommandIdentifiers.AUTO_ANC_STRENGTH]?.getOrNull(0)?.toFloat() ?: 50f))
    }
    val scope = rememberCoroutineScope()
    var job by remember { mutableStateOf<Job?>(null) }
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(28.dp))
            .background(cardColor())
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Icon(painterResource(R.drawable.ic_w_auto_awesome), null, tint = MaterialTheme.colorScheme.primary)
            Text(stringResource(R.string.adaptive_audio), style = MaterialTheme.typography.titleMedium)
        }
        Text(stringResource(R.string.ui_adaptive_desc), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Slider(
            value = value,
            onValueChange = {
                value = it
                job?.cancel()
                job = scope.launch {
                    delay(150)
                    viewModel.setControlCommandValue(
                        ControlCommandIdentifiers.AUTO_ANC_STRENGTH, byteArrayOf((100 - it).toInt().toByte())
                    )
                }
            },
            valueRange = 0f..100f
        )
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(stringResource(R.string.ui_less_noise), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(stringResource(R.string.ui_more_noise), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

class ControlsNavigation(
    val leftLongPress: () -> Unit,
    val rightLongPress: () -> Unit,
    val headGestures: () -> Unit,
    val callControl: (String) -> Unit,
)

@Composable
fun ControlsScreen(viewModel: AirPodsViewModel, topPadding: Dp, nav: ControlsNavigation) {
    val state by viewModel.uiState.collectAsState()
    val caps = state.capabilities
    val model = state.instance?.model

    SubPage(topPadding) {
        if (Capability.STEM_CONFIG in caps) {
            item { SectionHeader(stringResource(R.string.ui_press_and_hold), Modifier.padding(top = 0.dp)) }
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    BudCard(
                        side = stringResource(R.string.left), action = state.leftAction,
                        image = model?.leftBudsRes ?: R.drawable.airpods_pro_2_left,
                        onClick = nav.leftLongPress,
                        modifier = Modifier.weight(1f)
                    )
                    BudCard(
                        side = stringResource(R.string.right), action = state.rightAction,
                        image = model?.rightBudsRes ?: R.drawable.airpods_pro_2_right,
                        onClick = nav.rightLongPress,
                        modifier = Modifier.weight(1f)
                    )
                }
            }
            item {
                val config = state.controlStates[ControlCommandIdentifiers.LISTENING_MODE_CONFIGS]?.getOrNull(0)?.toInt() ?: 0
                val modes = listOfNotNull(
                    Triple(0x02, R.string.ui_mode_anc, R.drawable.ic_w_noise_control_on),
                    if (Capability.ADAPTIVE_AUDIO in caps) Triple(0x08, R.string.ui_mode_adaptive, R.drawable.ic_w_auto_awesome) else null,
                    Triple(0x04, R.string.ui_mode_transparency, R.drawable.ic_w_hearing),
                    if (state.offListeningMode) Triple(0x01, R.string.ui_mode_off, R.drawable.ic_w_noise_control_off) else null,
                )
                Column(
                    modifier = Modifier
                        .padding(top = 8.dp)
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(24.dp))
                        .background(cardColor())
                        .padding(horizontal = 18.dp, vertical = 16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Text(stringResource(R.string.ui_noise_control_cycles), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        modes.forEach { (bit, label, icon) ->
                            ChoiceChip(stringResource(label), selected = config and bit != 0, onClick = { viewModel.toggleListeningMode(bit) }, icon = icon)
                        }
                    }
                }
            }
        }

        val swipe = Capability.SWIPE_FOR_VOLUME in caps
        val head = Capability.HEAD_GESTURES in caps
        if (swipe || head) {
            item { SectionHeader(stringResource(R.string.ui_gestures)) }
            item {
                SettingsGroup(
                    rows = arrayOf(
                        if (swipe) { m -> SwitchRow(m, stringResource(R.string.ui_swipe_volume), stringResource(R.string.ui_swipe_volume_desc),
                            checked = state.flag(ControlCommandIdentifiers.VOLUME_SWIPE_MODE),
                            onCheckedChange = { viewModel.setControlCommandBoolean(ControlCommandIdentifiers.VOLUME_SWIPE_MODE, it) }) } else null,
                        if (head) { m -> SettingsRow(m, stringResource(R.string.head_gestures), stringResource(R.string.ui_head_gestures_desc),
                            onClick = nav.headGestures) {
                            Text(stringResource(if (state.headGesturesEnabled) R.string.on else R.string.off),
                                style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Icon(painterResource(R.drawable.ic_ui_chevron_right), null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                        } } else null,
                    )
                )
            }
        }

        item { SectionHeader(stringResource(R.string.ui_calls)) }
        item {
            val bytes = state.controlStates[ControlCommandIdentifiers.CALL_MANAGEMENT_CONFIG]
            val flipped = bytes?.getOrNull(1) == 0x02.toByte()
            val once = stringResource(R.string.press_once)
            val twice = stringResource(R.string.press_twice)
            val mute = stringResource(R.string.mute_unmute)
            val hangUp = stringResource(R.string.hang_up)
            SettingsGroup(
                rows = arrayOf(
                    { m -> SettingsRow(m, stringResource(R.string.answer_call), trailing = { ValueText(once) }) },
                    { m -> SettingsRow(m, mute, onClick = { nav.callControl(mute) }) {
                        ValueText(if (flipped) twice else once); Chevron()
                    } },
                    { m -> SettingsRow(m, stringResource(R.string.ui_end_call), onClick = { nav.callControl(hangUp) }) {
                        ValueText(if (flipped) once else twice); Chevron()
                    } },
                )
            )
        }
    }
}

@Composable
private fun ValueText(text: String) =
    Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)

@Composable
private fun Chevron() =
    Icon(painterResource(R.drawable.ic_ui_chevron_right), null, tint = MaterialTheme.colorScheme.onSurfaceVariant)

@Composable
private fun stemActionLabel(action: StemAction) = stringResource(
    when (action) {
        StemAction.CYCLE_NOISE_CONTROL_MODES -> R.string.noise_control
        StemAction.DIGITAL_ASSISTANT -> R.string.digital_assistant
        StemAction.PLAY_PAUSE -> R.string.play_pause
        StemAction.NEXT_TRACK -> R.string.next_track
        StemAction.PREVIOUS_TRACK -> R.string.previous_track
    }
)

@Composable
private fun BudCard(side: String, action: StemAction, @DrawableRes image: Int, onClick: () -> Unit, modifier: Modifier) {
    Column(
        modifier = modifier
            .clip(RoundedCornerShape(28.dp))
            .background(cardColor())
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        Box(
            Modifier.fillMaxWidth().height(112.dp).clip(RoundedCornerShape(20.dp)).background(MaterialTheme.colorScheme.surfaceContainer),
            contentAlignment = Alignment.Center
        ) {
            Image(painterResource(image), contentDescription = null, modifier = Modifier.height(96.dp))
        }
        Text(side, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(stemActionLabel(action), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Medium)
    }
}
