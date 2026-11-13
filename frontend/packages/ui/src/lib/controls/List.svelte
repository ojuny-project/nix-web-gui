<script lang="ts">
  export let id: string;
  export let items: string[];
  export let packages = false;
  export let disabled = false;
  export let onchange: (value: string[]) => void;

  function update(index: number, value: string) {
    const next = [...items];
    next[index] = value;
    onchange(next);
  }

  function remove(index: number) {
    onchange(items.filter((_, current) => current !== index));
  }

  function add() {
    onchange([...items, packages ? "pkgs." : ""]);
  }
</script>

<div class="list-field" data-list={id}>
  {#each items as item, index (index)}
    <div class="list-row">
      <input
        value={item}
        {disabled}
        placeholder={packages ? "pkgs.git" : "value"}
        oninput={(event) => update(index, event.currentTarget.value)}
      />
      <button type="button" class="ghost" {disabled} onclick={() => remove(index)}>
        Remove
      </button>
    </div>
  {/each}
  <button type="button" class="ghost" {disabled} onclick={add}>Add item</button>
</div>
