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

import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import me.kavishdevar.librepods.data.AirPodsBase
import me.kavishdevar.librepods.data.Battery
import me.kavishdevar.librepods.services.CompanionSession
import me.kavishdevar.librepods.services.CompanionSessions
import me.kavishdevar.librepods.services.ServiceManager
import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.bluetooth.AACPManager.Companion.ControlCommandIdentifiers
import me.kavishdevar.librepods.data.AirPodsMax
import me.kavishdevar.librepods.data.BatteryComponent
import me.kavishdevar.librepods.data.BatteryStatus
import me.kavishdevar.librepods.data.Capability
import me.kavishdevar.librepods.presentation.components.BatteryBar
import me.kavishdevar.librepods.presentation.components.BatteryTile
import me.kavishdevar.librepods.presentation.components.ChoiceChip
import me.kavishdevar.librepods.presentation.components.LeadTone
import me.kavishdevar.librepods.presentation.components.ListeningModeGroup
import me.kavishdevar.librepods.presentation.components.RadioRow
import me.kavishdevar.librepods.presentation.components.SectionHeader
import me.kavishdevar.librepods.presentation.components.SettingsGroup
import me.kavishdevar.librepods.presentation.components.SettingsRow
import me.kavishdevar.librepods.presentation.components.SwitchRow
import me.kavishdevar.librepods.presentation.components.ToggleTile
import me.kavishdevar.librepods.presentation.components.cardColor
import me.kavishdevar.librepods.presentation.components.isCharging
import me.kavishdevar.librepods.presentation.components.listeningModeOptions
import me.kavishdevar.librepods.presentation.viewmodel.AirPodsUiState
import me.kavishdevar.librepods.presentation.viewmodel.AirPodsViewModel

/** Where the home screen can navigate to. */
class HomeNavigation(
    val rename: () -> Unit,
    val sound: () -> Unit,
    val controls: () -> Unit,
    val accessibility: () -> Unit,
    val microphone: () -> Unit,
    val version: () -> Unit,
    val troubleshooting: () -> Unit,
    val purchase: () -> Unit,
)

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun HomeRoute(viewModel: AirPodsViewModel, topPadding: androidx.compose.ui.unit.Dp, nav: HomeNavigation) {
    val state by viewModel.uiState.collectAsState()
    val bottomPadding = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding() + 24.dp
    Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceContainer)) {
        // Give the service a moment to (re)connect before showing "Not connected", so the
        // screen doesn't flash disconnected → connected on launch.
        var graceOver by remember { mutableStateOf(false) }
        LaunchedEffect(Unit) {
            delay(3000)
            // Keep waiting while the service is still busy connecting.
            while (me.kavishdevar.librepods.presentation.widgets.WidgetRenderer.isChecking()) delay(500)
            graceOver = true
        }
        when {
            state.isLocallyConnected -> MultiDeviceHome(state, viewModel, nav, topPadding, bottomPadding)
            !graceOver -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    LoadingIndicator(Modifier.size(64.dp))
                    Text(stringResource(R.string.ui_reconnecting), style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            else -> DisconnectedHome(state, viewModel, nav, topPadding)
        }
    }
}

/** Main device plus any companions connected alongside it, with a switcher when there's more than one. */
@Composable
private fun MultiDeviceHome(
    state: AirPodsUiState,
    viewModel: AirPodsViewModel,
    nav: HomeNavigation,
    topPadding: androidx.compose.ui.unit.Dp,
    bottomPadding: androidx.compose.ui.unit.Dp,
) {
    val tick by CompanionSessions.changes.collectAsState()
    val companions = remember(tick) { CompanionSessions.all() }
    var selected by remember { mutableStateOf<String?>(null) }
    val companion = companions.find { it.mac == selected }
    if (selected != null && companion == null) selected = null

    val header: @Composable () -> Unit = {
        if (companions.isNotEmpty()) {
            DeviceSwitcher(
                entries = listOf(
                    DeviceEntry(null, state.deviceName, state.instance?.model is AirPodsMax, state.battery)
                ) + companions.map {
                    DeviceEntry(it.mac, it.name ?: "AirPods", it.instance?.model is AirPodsMax, it.batteryNotification.getBattery())
                },
                selected = selected,
                onSelect = { selected = it }
            )
        }
    }
    if (companion == null) {
        ConnectedHome(state, viewModel, nav, topPadding, bottomPadding, header)
    } else {
        CompanionHome(companion, tick, topPadding, bottomPadding, header, onMadeMain = { selected = null })
    }
}

private data class DeviceEntry(val mac: String?, val name: String, val isMax: Boolean, val battery: List<Battery>)

@Composable
private fun DeviceSwitcher(entries: List<DeviceEntry>, selected: String?, onSelect: (String?) -> Unit) {
    LazyRow(
        modifier = Modifier.padding(bottom = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        items(entries.size) { i ->
            val entry = entries[i]
            val isSelected = entry.mac == selected
            val level = entry.battery
                .filter { it.status != BatteryStatus.DISCONNECTED && it.component != BatteryComponent.CASE && it.level > 0 }
                .minOfOrNull { it.level }
            Row(
                modifier = Modifier
                    .clip(RoundedCornerShape(20.dp))
                    .background(if (isSelected) MaterialTheme.colorScheme.primaryContainer else cardColor())
                    .clickable { onSelect(entry.mac) }
                    .padding(start = 12.dp, end = 16.dp, top = 10.dp, bottom = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                val fg = if (isSelected) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant
                Icon(
                    painterResource(if (entry.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds), null,
                    tint = fg, modifier = Modifier.size(20.dp)
                )
                Column {
                    Text(
                        entry.name, style = MaterialTheme.typography.labelLarge, maxLines = 1,
                        overflow = TextOverflow.Ellipsis, modifier = Modifier.widthIn(max = 160.dp),
                        color = if (isSelected) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface
                    )
                    if (level != null) Text("$level%", style = MaterialTheme.typography.bodySmall, color = fg)
                }
            }
        }
    }
}

/** A pair connected next to the main one: battery and listening mode, plus a way to make it the main device. */
@Composable
private fun CompanionHome(
    session: CompanionSession,
    tick: Long,
    topPadding: androidx.compose.ui.unit.Dp,
    bottomPadding: androidx.compose.ui.unit.Dp,
    header: @Composable () -> Unit,
    onMadeMain: () -> Unit,
) {
    val context = LocalContext.current
    val model = session.instance?.model
    val isMax = model is AirPodsMax
    val commands = remember(tick) { session.aacpManager.controlCommandStatusList.toList() }
    val mode = commands.find { it.identifier == ControlCommandIdentifiers.LISTENING_MODE }?.value?.firstOrNull()?.toInt() ?: 2
    val allowOff = commands.find { it.identifier == ControlCommandIdentifiers.ALLOW_OFF_OPTION }?.value?.firstOrNull() == 0x01.toByte()
    val battery = remember(tick) { session.batteryNotification.getBattery() }

    LazyColumn(Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
        item { Spacer(Modifier.height(topPadding)) }
        item { header() }
        item {
            Column(Modifier.padding(horizontal = 8.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text(
                    stringResource(R.string.ui_also_connected),
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Text(session.name ?: "AirPods", style = MaterialTheme.typography.headlineMedium)
            }
        }
        item { Hero(model, isMax) }
        item { if (isMax) MaxBattery(battery) else BudsBattery(battery) }
        if (model == null || Capability.LISTENING_MODE in model.capabilities) {
            item { SectionHeader(stringResource(R.string.noise_control)) }
            item {
                ListeningModeGroup(
                    options = listeningModeOptions(allowOff, model == null || Capability.ADAPTIVE_AUDIO in model.capabilities),
                    selected = mode,
                    onSelect = { newMode ->
                        me.kavishdevar.librepods.presentation.widgets.WidgetRenderer.setModeOptimistically(context, session.device.address, newMode)
                        Thread {
                            session.aacpManager.sendControlCommand(ControlCommandIdentifiers.LISTENING_MODE.value, newMode.toByte())
                            CompanionSessions.changed(context)
                        }.start()
                    }
                )
            }
        }
        item {
            Column(
                modifier = Modifier
                    .padding(top = 16.dp)
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(24.dp))
                    .background(cardColor())
                    .padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Text(stringResource(R.string.ui_main_device_title), style = MaterialTheme.typography.titleMedium)
                Text(
                    stringResource(R.string.ui_main_device_desc),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Button(onClick = {
                    ServiceManager.getService()?.makeMain(session.device)
                    onMadeMain()
                }) { Text(stringResource(R.string.ui_make_main_device)) }
            }
        }
        item { Spacer(Modifier.height(bottomPadding)) }
    }
}

private fun AirPodsUiState.flag(id: ControlCommandIdentifiers) = controlStates[id]?.getOrNull(0) == 0x01.toByte()

@Composable
private fun ConnectedHome(
    state: AirPodsUiState,
    viewModel: AirPodsViewModel,
    nav: HomeNavigation,
    topPadding: androidx.compose.ui.unit.Dp,
    bottomPadding: androidx.compose.ui.unit.Dp,
    header: @Composable () -> Unit = {},
) {
    val model = state.instance?.model
    val isMax = model is AirPodsMax
    val caps = state.capabilities
    val mode = state.controlStates[ControlCommandIdentifiers.LISTENING_MODE]?.getOrNull(0)?.toInt() ?: state.ancMode

    LazyColumn(Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
        item { Spacer(Modifier.height(topPadding)) }
        item { header() }

        item {
            Column(Modifier.padding(horizontal = 8.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Row(
                    modifier = Modifier
                        .clip(RoundedCornerShape(14.dp))
                        .background(MaterialTheme.colorScheme.primaryContainer)
                        .padding(horizontal = 12.dp, vertical = 5.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    Icon(
                        painterResource(R.drawable.ic_ui_bluetooth_connected), null,
                        tint = MaterialTheme.colorScheme.onPrimaryContainer, modifier = Modifier.size(16.dp)
                    )
                    Text(
                        stringResource(R.string.ui_connected_model, model?.displayName?.takeIf { isMax } ?: model?.name ?: "AirPods"),
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                }
                Text(state.deviceName, style = MaterialTheme.typography.headlineMedium)
            }
        }

        item { Hero(model, isMax) }

        item {
            if (isMax) MaxBattery(state.battery) else BudsBattery(state.battery)
        }

        if (Capability.LISTENING_MODE in caps) {
            item { SectionHeader(stringResource(R.string.noise_control)) }
            item {
                val ctx = LocalContext.current
                ListeningModeGroup(
                    options = listeningModeOptions(state.offListeningMode, Capability.ADAPTIVE_AUDIO in caps),
                    selected = mode,
                    onSelect = {
                        me.kavishdevar.librepods.presentation.widgets.WidgetRenderer.setModeOptimistically(ctx, null, it)
                        viewModel.setControlCommandInt(ControlCommandIdentifiers.LISTENING_MODE, it)
                    }
                )
            }
        }

        val tiles = listOfNotNull(
            if (Capability.CONVERSATION_AWARENESS in caps) Triple(
                R.string.conversational_awareness, R.drawable.ic_ui_record_voice_over, ControlCommandIdentifiers.CONVERSATION_DETECT_CONFIG
            ) else null,
            if (Capability.ADAPTIVE_VOLUME in caps) Triple(
                R.string.personalized_volume, R.drawable.ic_ui_volume_up, ControlCommandIdentifiers.ADAPTIVE_VOLUME_CONFIG
            ) else null,
        )
        if (tiles.isNotEmpty()) {
            item {
                Row(Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    tiles.forEach { (label, icon, id) ->
                        ToggleTile(
                            label = stringResource(label),
                            icon = icon,
                            checked = state.flag(id),
                            onCheckedChange = { viewModel.setControlCommandBoolean(id, it) },
                            modifier = Modifier.weight(1f)
                        )
                    }
                }
            }
        }

        if (isMax) {
            item { SectionHeader(stringResource(R.string.ui_noise_control_button)) }
            item { NoiseControlButtonCard(state, viewModel) }
            item { SectionHeader(stringResource(R.string.ui_digital_crown)) }
            item {
                val crown = state.controlStates[ControlCommandIdentifiers.CROWN_ROTATION_DIRECTION]?.getOrNull(0)?.toInt() ?: 1
                SettingsGroup(
                    rows = arrayOf(
                        { m -> RadioRow(m, stringResource(R.string.ui_back_to_front), stringResource(R.string.ui_back_to_front_desc), crown != 2) {
                            viewModel.setControlCommandInt(ControlCommandIdentifiers.CROWN_ROTATION_DIRECTION, 1)
                        } },
                        { m -> RadioRow(m, stringResource(R.string.ui_front_to_back), stringResource(R.string.ui_front_to_back_desc), crown == 2) {
                            viewModel.setControlCommandInt(ControlCommandIdentifiers.CROWN_ROTATION_DIRECTION, 2)
                        } },
                    )
                )
            }
        }

        item { SectionHeader(stringResource(R.string.ui_settings_section)) }
        item {
            val hasHearing = Capability.HEARING_AID in caps || Capability.PPE in caps
            SettingsGroup(
                rows = arrayOf(
                    { m -> SettingsRow(m, stringResource(R.string.ui_sound),
                        stringResource(if (isMax) R.string.ui_sound_desc_max else R.string.ui_sound_desc),
                        icon = R.drawable.ic_ui_graphic_eq, onClick = nav.sound) },
                    if (!isMax) { m -> SettingsRow(m, stringResource(R.string.ui_controls), stringResource(R.string.ui_controls_desc),
                        icon = R.drawable.ic_ui_touch_app, onClick = nav.controls) } else null,
                    if (hasHearing) { m -> SettingsRow(m, stringResource(R.string.hearing_health), stringResource(R.string.ui_hearing_desc),
                        icon = R.drawable.ic_w_hearing, tone = LeadTone.Tertiary, onClick = {}, wip = true) } else null,
                    { m -> SettingsRow(m, stringResource(R.string.accessibility), stringResource(R.string.ui_accessibility_desc),
                        icon = R.drawable.ic_ui_accessibility_new, tone = LeadTone.Secondary, onClick = nav.accessibility) },
                )
            )
        }

        item { SectionHeader(stringResource(R.string.ui_device_section)) }
        item {
            val micMode = when (state.controlStates[ControlCommandIdentifiers.MIC_MODE]?.getOrNull(0)?.toInt() ?: 0) {
                1 -> R.string.microphone_always_right
                2 -> R.string.microphone_always_left
                else -> R.string.microphone_automatic
            }
            SettingsGroup(
                rows = arrayOf(
                    { m -> SettingsRow(m, stringResource(R.string.name), state.deviceName, onClick = nav.rename) {
                        Icon(painterResource(R.drawable.ic_ui_edit), null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                    } },
                    if (!isMax) { m -> SettingsRow(m, stringResource(R.string.microphone_mode), stringResource(micMode), onClick = nav.microphone) } else null,
                    { m -> SwitchRow(m,
                        stringResource(if (isMax) R.string.ui_head_detection else R.string.ear_detection),
                        if (isMax) stringResource(R.string.ui_head_detection_desc) else null,
                        checked = state.automaticEarDetectionEnabled,
                        onCheckedChange = viewModel::setAutomaticEarDetectionEnabled) },
                    if (Capability.SLEEP_DETECTION in caps) { m -> SwitchRow(m, stringResource(R.string.sleep_detection), stringResource(R.string.ui_sleep_desc),
                        checked = state.flag(ControlCommandIdentifiers.SLEEP_DETECTION_CONFIG),
                        onCheckedChange = { viewModel.setControlCommandBoolean(ControlCommandIdentifiers.SLEEP_DETECTION_CONFIG, it) },
                        enabled = state.isPremium) } else null,
                    if (!isMax) { m -> SwitchRow(m, stringResource(R.string.optimized_charging), null,
                        checked = state.dynamicEndOfCharge, onCheckedChange = viewModel::setDynamicEndOfCharge) } else null,
                    if (!state.isPremium) { m -> SettingsRow(m, stringResource(R.string.ui_unlock), onClick = nav.purchase) } else null,
                    { m -> SettingsRow(m, stringResource(R.string.ui_about), state.version1.ifEmpty { null }, onClick = nav.version) },
                )
            )
        }

        item {
            Box(Modifier.fillMaxWidth().padding(top = 24.dp), contentAlignment = Alignment.Center) {
                OutlinedButton(onClick = viewModel::disconnect) {
                    Text(stringResource(R.string.disconnect))
                }
            }
        }
        item { Spacer(Modifier.height(bottomPadding)) }
    }
}

@Composable
private fun Hero(model: AirPodsBase?, isMax: Boolean) {
    Box(
        modifier = Modifier
            .padding(top = 16.dp)
            .fillMaxWidth()
            .height(if (isMax) 200.dp else 180.dp)
            .clip(RoundedCornerShape(40.dp))
            .background(MaterialTheme.colorScheme.primaryContainer),
        contentAlignment = Alignment.Center
    ) {
        Box(
            Modifier
                .size(300.dp)
                .align(Alignment.TopEnd)
                .offset(x = 90.dp, y = (-110).dp)
                .clip(CircleShape)
                .background(cardColor().copy(alpha = 0.3f))
        )
        if (isMax) {
            Image(painterResource(R.drawable.airpods_max_art), contentDescription = model?.displayName, modifier = Modifier.size(160.dp))
        } else {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                Image(painterResource(model?.caseRes ?: R.drawable.airpods_pro_2_case), null, modifier = Modifier.width(130.dp))
                Image(painterResource(model?.budsRes ?: R.drawable.airpods_pro_2_buds), model?.name, modifier = Modifier.width(115.dp))
            }
        }
    }
}

@Composable
private fun BudsBattery(batteries: List<Battery>) {
    val available = batteries.filter { it.status != BatteryStatus.DISCONNECTED }
    val labels = mapOf(
        BatteryComponent.LEFT to R.string.left,
        BatteryComponent.RIGHT to R.string.right,
        BatteryComponent.CASE to R.string.case_alt,
    )
    if (available.isEmpty()) return
    Row(Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        available.sortedBy { listOf(BatteryComponent.LEFT, BatteryComponent.RIGHT, BatteryComponent.CASE).indexOf(it.component) }
            .forEach { BatteryTile(stringResource(labels[it.component] ?: R.string.ui_battery), it, Modifier.weight(1f)) }
    }
}

@Composable
private fun MaxBattery(batteries: List<Battery>) {
    val battery = batteries.firstOrNull { it.status != BatteryStatus.DISCONNECTED && it.component != BatteryComponent.CASE } ?: return
    Column(
        modifier = Modifier
            .padding(top = 8.dp)
            .fillMaxWidth()
            .clip(RoundedCornerShape(28.dp))
            .background(cardColor())
            .padding(horizontal = 20.dp, vertical = 18.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.ui_battery), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text("${battery.level}%", style = MaterialTheme.typography.headlineMedium)
        BatteryBar(battery.level, charging = battery.isCharging(), height = 10.dp)
    }
}

@Composable
private fun NoiseControlButtonCard(state: AirPodsUiState, viewModel: AirPodsViewModel) {
    val config = state.controlStates[ControlCommandIdentifiers.LISTENING_MODE_CONFIGS]?.getOrNull(0)?.toInt() ?: 0
    val modes = listOf(
        Triple(0x02, R.string.noise_cancellation, R.drawable.ic_w_noise_control_on),
        Triple(0x04, R.string.transparency, R.drawable.ic_w_hearing),
        Triple(0x01, R.string.off, R.drawable.ic_w_noise_control_off),
    )
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(24.dp))
            .background(cardColor())
            .padding(horizontal = 18.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.ui_cycles_between), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            modes.forEach { (bit, label, icon) ->
                ChoiceChip(stringResource(label), selected = config and bit != 0, onClick = { viewModel.toggleListeningMode(bit) }, icon = icon)
            }
        }
    }
}

@Composable
private fun DisconnectedHome(
    state: AirPodsUiState,
    viewModel: AirPodsViewModel,
    nav: HomeNavigation,
    topPadding: androidx.compose.ui.unit.Dp,
) {
    val model = state.instance?.model
    var connecting by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().padding(horizontal = 16.dp).padding(top = topPadding + 8.dp)) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(220.dp)
                .clip(RoundedCornerShape(40.dp))
                .background(MaterialTheme.colorScheme.surfaceContainerHigh),
            contentAlignment = Alignment.Center
        ) {
            val grey = ColorFilter.colorMatrix(ColorMatrix().apply { setToSaturation(0f) })
            Image(
                painterResource(if (model is AirPodsMax) R.drawable.airpods_max_art else model?.caseRes ?: R.drawable.airpods_pro_2_case),
                contentDescription = null,
                colorFilter = grey,
                modifier = Modifier.width(160.dp).alpha(0.55f)
            )
            Row(
                modifier = Modifier
                    .align(Alignment.TopStart)
                    .padding(20.dp)
                    .clip(RoundedCornerShape(8.dp))
                    .background(MaterialTheme.colorScheme.surfaceBright)
                    .padding(horizontal = 12.dp, vertical = 6.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(painterResource(R.drawable.ic_w_bluetooth_disabled), null, Modifier.size(16.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(stringResource(R.string.ui_not_connected), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        Column(Modifier.padding(start = 8.dp, end = 8.dp, top = 24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(state.deviceName, style = MaterialTheme.typography.headlineSmall)
            Text(stringResource(R.string.ui_not_connected_hint), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Row(Modifier.padding(top = 24.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Button(
                onClick = { connecting = true; viewModel.reconnectFromSavedMac() },
                modifier = Modifier.weight(1f).height(48.dp)
            ) {
                Icon(painterResource(R.drawable.ic_w_bluetooth_fill), null)
                Spacer(Modifier.width(8.dp))
                Text(stringResource(if (connecting) R.string.ui_reconnecting else R.string.ui_connect), style = MaterialTheme.typography.labelLarge)
            }
            FilledTonalIconButton(onClick = nav.troubleshooting, modifier = Modifier.size(48.dp)) {
                Icon(painterResource(R.drawable.ic_ui_build), contentDescription = stringResource(R.string.troubleshooting))
            }
        }
        TextButton(onClick = viewModel::activateDemoMode, modifier = Modifier.padding(top = 8.dp)) {
            Text(stringResource(R.string.ui_try_demo))
        }
    }
}
