<script lang="ts">
  import { Ellipsis, Pause, Play } from '@lucide/svelte';
  import { Button, DropdownMenu, EmptyState, Group, IconButton, MenuItem, Row, confirmAlert, toast } from '@speaktype/ui';
  import ModelMenu from '../../../lib/app/ModelMenu.svelte';
  import { copyToClipboard, deleteHistoryEntry, getHistoryAudio, reprocessHistoryEntry } from '../../../lib/ipc';
  import { navigate } from '../../router.svelte';
  import { commands } from '../../../lib/bindings';
  const { pasteLast } = commands;
  import { detailMetaOf, titleOf, type Dictation, type Entry } from './history';

  const COPIED_MS = 1500;

  let { item, newest, onchange }: { item: Entry; newest: boolean; onchange: () => void } = $props();

  let copied = $state(false);
  let playing = $state(false);
  let reprocessing = $state(false);
  let audio: HTMLAudioElement | undefined;

  const hasAudio = $derived(item.kind === 'dictation' && !!item.entry.audio_file);
  const message = (error: unknown) => (error as { message?: string })?.message ?? `${error}`;

  $effect(() => () => {
    audio?.pause();
    if (audio) URL.revokeObjectURL(audio.src);
  });

  async function copy(entry: Dictation) {
    await copyToClipboard(entry.entry.text);
    copied = true;
    setTimeout(() => (copied = false), COPIED_MS);
  }

  async function pasteAgain() {
    try {
      await pasteLast();
    } catch (error) {
      toast.show({ message: `Couldn't paste. ${message(error)}` });
    }
  }

  async function togglePlay(entry: Dictation) {
    if (!audio) {
      const wav = new Uint8Array(await getHistoryAudio(entry.index));
      audio = new Audio(URL.createObjectURL(new Blob([wav], { type: 'audio/wav' })));
      audio.onended = () => (playing = false);
    }
    if (playing) audio.pause();
    else await audio.play();
    playing = !playing;
  }

  async function reprocess(entry: Dictation) {
    reprocessing = true;
    try {
      await reprocessHistoryEntry(entry.index);
      onchange();
    } catch (error) {
      toast.show({ message: `Couldn't redo this dictation. ${message(error)}` });
    } finally {
      reprocessing = false;
    }
  }

  async function remove(entry: Dictation) {
    const confirmed = await confirmAlert({
      title: 'Delete this dictation?',
      message: 'It and its saved audio are removed from this Mac.',
      confirmLabel: 'Delete',
      destructive: true,
    });
    if (!confirmed) return;
    await deleteHistoryEntry(entry.index);
    onchange();
  }
</script>

{#if item.kind === 'dictation'}
  <article>
    <header>
      <div class="heading">
        <h2>{titleOf(item.entry.text)}</h2>
        <p>{detailMetaOf(item, new Date())}</p>
      </div>
      <div class="actions">
        <Button variant="primary" onclick={() => copy(item)}>{copied ? 'Copied' : 'Copy'}</Button>
        {#if newest}<Button onclick={pasteAgain}>Paste again</Button>{/if}
        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}<IconButton {...props} label="More" icon={Ellipsis} />{/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content>
            <MenuItem label="Delete" destructive onSelect={() => remove(item)} />
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>
    </header>
    <Group><p class="text">{item.entry.text}</p></Group>
    {#if hasAudio}
      <Group>
        <Row kind="control" label="Recording" caption="Saved with this dictation">
          {#snippet trailing()}
            <Button size={28} onclick={() => togglePlay(item)}>
              {#if playing}<Pause size={13} aria-hidden="true" />Pause{:else}<Play size={13} aria-hidden="true" />Play{/if}
            </Button>
          {/snippet}
        </Row>
        <Row kind="control" label="Redo with" caption="The new text replaces this one">
          {#snippet trailing()}
            <ModelMenu trigger="compact" onopenproviders={() => navigate('providers')} />
            <Button size={28} busy={reprocessing} onclick={() => reprocess(item)}>Redo</Button>
          {/snippet}
        </Row>
      </Group>
    {/if}
  </article>
{:else}
  <EmptyState
    title={item.session.name}
    message="The Timeline for this session is coming soon. Its voice, marks and copied text are saved on this Mac."
  />
{/if}

<style>
  article {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 28px;
    user-select: text;
  }
  header {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  /* One line, truncated: the full text is right below it. */
  h2 {
    margin: 0;
    overflow: hidden;
    font-size: 17px;
    font-weight: 700;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .heading p {
    margin: 6px 0 0;
    color: var(--st-text-secondary);
    font-size: var(--st-font-info);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .text {
    margin: 0;
    padding: 16px;
    font-size: var(--st-font-body);
    line-height: 1.5;
    white-space: pre-wrap;
  }
</style>
