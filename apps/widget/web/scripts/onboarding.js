const RECOMMENDED = 'small.en';
const SETTINGS_MODELS_HINT = ' You can also download or switch models anytime in Settings (menu bar icon → Models).';

let status = {
  microphone: false,
  accessibility: false,
  model: false,
  active_model: '',
  exe_path: '',
  is_dev: false,
  is_bundled: false,
  responsible_app: '',
  responsible_app_path: '',
};
let lastStatusKey = '';
let micPromptRequested = false;
let micSettingsOpened = false;
let accSettingsOpened = false;
let modelGridShown = false;

const $ = id => document.getElementById(id);

const modelGrid = new ModelGrid({
  container: 'model-grid',
  mode: 'onboarding',
  recommendedModel: RECOMMENDED,
  getActiveModel() {
    return status.model ? status.active_model : '';
  },
  hooks: {
    onReady() {
      poll(true);
    },
    onError(message) {
      $('desc-model').textContent = (message || 'Model operation failed') + ' — try again.';
      modelGrid.refresh(true);
      render({ refreshModels: false });
    },
    onPaused() {
      modelGrid.refresh(true);
      poll(true);
    },
    onRefresh() {
      updateModelStepLayout();
    },
  },
});

function statusKey(s) {
  return [
    s.microphone,
    s.accessibility,
    s.model,
    s.active_model || '',
    s.is_dev ? '1' : '0',
    s.is_bundled ? '1' : '0',
    s.exe_path || '',
    s.responsible_app || '',
  ].join('|');
}

function setStep(el, state) {
  el.classList.remove('locked', 'active', 'done');
  el.classList.add(state);
}

function updateModelStepLayout() {
  const modelStep = $('step-model');
  const host = $('model-grid-host');
  const scroll = $('steps-scroll');
  const showingList = modelStep.classList.contains('active')
    && !modelStep.classList.contains('done')
    && !host.hidden;
  modelStep.classList.toggle('expand', showingList);
  scroll.classList.toggle('model-active', showingList);
  syncScrollMetrics();
}

function syncScrollMetrics() {
  const header = document.querySelector('.header');
  const footer = document.querySelector('.footer');
  if (!header || !footer) return;
  const bodyPad = 40;
  const minH = window.innerHeight - header.offsetHeight - footer.offsetHeight - bodyPad;
  document.documentElement.style.setProperty('--onboarding-model-min-h', `${Math.max(minH, 200)}px`);
}

function updateRelaunchHints(micDone, accDone) {
  const showMicRelaunch = !micDone && micSettingsOpened;
  $('hint-mic-relaunch').hidden = !showMicRelaunch;
  $('btn-quit-mic').hidden = !showMicRelaunch;

  const isDev = !!status.is_dev;
  const unbundled = isDev && !status.is_bundled;
  const showAcc = !accDone;
  $('hint-acc-relaunch').hidden = !showAcc || isDev;
  $('hint-acc-dev').hidden = !showAcc || !isDev;
  $('hint-acc-path').hidden = !showAcc || !status.exe_path;
  $('btn-quit-acc').hidden = !showAcc;

  if (showAcc && unbundled) {
    const owner = status.responsible_app || 'the terminal app that launched it';
    $('hint-acc-dev').textContent =
      'This is a bare executable, so macOS assigns Accessibility to ' + owner + ' rather than to SpeakType. ' +
      'Run "make dev" instead — it launches a real SpeakType Dev app that holds its own grant.';
  } else if (showAcc && isDev) {
    $('hint-acc-dev').textContent =
      'Dev build: enable "' + (status.responsible_app || 'SpeakType Dev') + '" in the Accessibility list, ' +
      'then Quit & Reopen. Remove any older entry with the same name first.';
  }

  if (showAcc && status.exe_path) {
    $('hint-acc-path').textContent = status.responsible_app_path
      ? 'Grant to: ' + status.responsible_app_path
      : status.exe_path;
  }
}

function render(opts = {}) {
  const refreshModels = opts.refreshModels === true;
  const micDone = status.microphone;
  const accDone = status.accessibility;
  const modelDone = status.model;

  setStep($('step-mic'), micDone ? 'done' : 'active');
  if (micDone) micSettingsOpened = false;
  $('desc-mic').textContent = micDone
    ? 'Microphone access granted.'
    : 'Allow microphone access when prompted, or enable SpeakType under Privacy & Security → Microphone in System Settings.';

  if (!micDone && !micPromptRequested) {
    micPromptRequested = true;
    ttipc.invoke('request_microphone_access').then(() => setTimeout(() => poll(true), 600));
  }

  setStep($('step-acc'), !micDone ? 'locked' : (accDone ? 'done' : 'active'));
  if (accDone) accSettingsOpened = false;
  $('desc-acc').textContent = accDone
    ? 'Accessibility access granted.'
    : (status.responsible_app
      ? 'Open System Settings → Privacy & Security → Accessibility and enable "' + status.responsible_app + '" (then Quit & Reopen).'
      : 'Open System Settings → Privacy & Security → Accessibility and enable the SpeakType entry for this install (then Quit & Reopen).');

  updateRelaunchHints(micDone, accDone);

  setStep($('step-model'), !accDone ? 'locked' : (modelDone ? 'done' : 'active'));

  const host = $('model-grid-host');
  if (modelDone) {
    $('desc-model').textContent = 'Model "' + status.active_model + '" is ready.' + SETTINGS_MODELS_HINT;
    host.hidden = true;
    modelGridShown = false;
  } else if (accDone) {
    $('desc-model').textContent = 'Pick a model to download. ' + RECOMMENDED + ' is recommended for most Macs.' + SETTINGS_MODELS_HINT;
    host.hidden = false;
    modelGrid.ensureListeners();
    // Always refresh when first shown, or when explicitly requested (pause/ready/poll force).
    if (!modelGridShown || refreshModels) {
      modelGridShown = true;
      modelGrid.refresh(true);
    }
  } else {
    host.hidden = true;
    modelGridShown = false;
  }

  updateModelStepLayout();
  $('start-btn').disabled = !(micDone && accDone && modelDone);
}

async function poll(forceRender) {
  try {
    const s = await ttipc.invoke('get_onboarding_status');
    if (s) {
      const sk = statusKey(s);
      if (forceRender || sk !== lastStatusKey) {
        lastStatusKey = sk;
        status = s;
        render({ refreshModels: forceRender });
      }
    }
  } catch (e) { /* ignore */ }
}

$('btn-mic-settings').onclick = () => {
  micSettingsOpened = true;
  updateRelaunchHints(status.microphone, status.accessibility);
  ttipc.openSystemPane('com.apple.preference.security?Privacy_Microphone');
};

$('btn-acc-settings').onclick = () => {
  accSettingsOpened = true;
  updateRelaunchHints(status.microphone, status.accessibility);
  ttipc.openSystemPane('com.apple.preference.security?Privacy_Accessibility');
};

function quitForRelaunch() {
  ttipc.quitApp();
}

$('btn-quit-mic').onclick = quitForRelaunch;
$('btn-quit-acc').onclick = quitForRelaunch;

$('start-btn').onclick = async () => {
  $('start-btn').disabled = true;
  $('start-btn').innerHTML = '<span class="spinner"></span> Starting…';
  try {
    await ttipc.invoke('finish_onboarding');
  } catch (e) {
    console.error(e);
  }
};

poll(true);
syncScrollMetrics();
window.addEventListener('resize', syncScrollMetrics);
setInterval(() => poll(false), 1500);
document.addEventListener('visibilitychange', () => {
  if (!document.hidden) poll(true);
});
