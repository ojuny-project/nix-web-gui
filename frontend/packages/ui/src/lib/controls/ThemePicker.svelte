<script lang="ts">
  import { SCHEMES } from "../theme";

  export let id: string;
  export let value: string;
  export let disabled = false;
  export let onchange: (value: string) => void;
</script>

<select
  {id}
  {value}
  {disabled}
  onchange={(event) => onchange(event.currentTarget.value)}
>
  {#each SCHEMES as scheme (scheme.id)}
    <option value={scheme.id}>
      {scheme.name} ({scheme.variant})
    </option>
  {/each}
</select>
