<script lang="ts">
  export let id: string;
  export let choices: string[];
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
  {#each choices as choice}
    <option value={choice}>{choice}</option>
  {/each}
</select>
