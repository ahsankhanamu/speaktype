<script lang="ts">
  import { LayerRoot } from '@speaktype/ui';
  import { pages } from './registry';

  function parse() {
    const [path, query = ''] = location.hash.slice(1).split('?');
    return { id: path.replace(/^\//, ''), theme: new URLSearchParams(query).get('theme') };
  }

  let route = $state(parse());

  const page = $derived(pages.find((p) => p.id === route.id));

  $effect(() => {
    if (route.theme) document.documentElement.dataset.theme = route.theme;
    else delete document.documentElement.dataset.theme;
  });
</script>

<svelte:window onhashchange={() => (route = parse())} />

<LayerRoot>
  {#if page}
    {#key page.id}
      <page.component />
    {/key}
  {:else}
    <nav class="index">
      <h1>@speaktype/ui</h1>
      {#each pages as p}<a href="#/{p.id}">{p.title}</a>{/each}
    </nav>
  {/if}
</LayerRoot>
