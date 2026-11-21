<script lang="ts">
  import { Check, Mic, TriangleAlert } from '@lucide/svelte';
  import { Halo, Toast, toast } from '@speaktype/ui';
  import { onMount } from 'svelte';

  onMount(() => toast.show({ message: 'Copied with 3 images', action: { label: 'Undo', run: () => {} }, durationMs: 3_600_000 }));
</script>

<div class="page">
  <section class="card">
    <p class="sec">Floating feedback</p>
    <p class="cap">Toasts confirm and offer undo. Popovers and menus share one tight shadow. Only the widget pill uses a soft status halo.</p>
    <div class="row center">
      <div class="inline-toast"><Toast /></div>
      <Halo tone="success"><span class="pill"><Check size={14} strokeWidth={2.4} color="var(--st-success)" />Pasted</span></Halo>
      <Halo tone="recording"><span class="pill"><Mic size={14} color="var(--st-danger)" />Listening</span></Halo>
      <Halo tone="active"><span class="pill"><Mic size={14} color="var(--st-accent)" />Getting ready</span></Halo>
      <Halo tone="warning"><span class="pill"><TriangleAlert size={14} color="var(--st-warning)" />No speech heard</span></Halo>
    </div>
  </section>
</div>

<style>
  .center {
    align-items: center;
    gap: 28px;
    min-height: 60px;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    box-sizing: border-box;
    height: 40px;
    padding: 0 16px;
    border-radius: 20px;
    background: rgba(40, 40, 42, 0.96);
    color: rgba(255, 255, 255, 0.88);
    font-size: var(--st-font-caption);
  }
</style>
