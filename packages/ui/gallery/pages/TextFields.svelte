<script lang="ts">
  import { SearchField, TextArea, TextField } from '@speaktype/ui';

  let multiline = $state('Two lines of text wrap here instead of scrolling sideways.');
</script>

<div class="page">
  <section class="card">
    <p class="sec">Text fields</p>
    <p class="cap">Pill for single-line fields and search; 12 pt corners for multi-line. Errors say what to do, under the field.</p>
    <div class="row">
      <div class="cell w220"><TextField placeholder="Session name" aria-label="Empty" /><p class="lbl">Empty</p></div>
      <div class="cell w220"><TextField value="Checkout bug" aria-label="Focused" data-force-state="focus" /><p class="lbl">Focused</p></div>
      <div class="cell w220"><TextField value="~/Desktop/missing" error="That folder doesn't exist" aria-label="Error" /></div>
    </div>
    <div class="row top">
      <div class="cell w220"><SearchField /><p class="lbl">Search</p></div>
      <div class="cell w220"><TextField value="Read only" disabled aria-label="Disabled" /><p class="lbl">Disabled</p></div>
      <div class="cell w220"><TextArea bind:value={multiline} aria-label="Multi-line" /><p class="lbl">Multi-line</p></div>
    </div>
  </section>
</div>

<style>
  .top {
    margin-top: 14px;
  }
</style>
