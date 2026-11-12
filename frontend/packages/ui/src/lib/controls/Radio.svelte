<script lang="ts">
  export let id: string;
  export let choices: string[];
  export let value: string;
  export let disabled = false;
  export let onchange: (value: string) => void;
</script>

<div class="radio-group" role="radiogroup">
  {#each choices as choice}
    <label class="radio">
      <input
        type="radio"
        name={id}
        value={choice}
        checked={value === choice}
        {disabled}
        onchange={() => onchange(choice)}
      />
      <span>{choice}</span>
    </label>
  {/each}
</div>
