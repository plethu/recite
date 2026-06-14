using Recite.Unity;
using Recite.Unity.GameObjects;
using UnityEngine;

public sealed class BasicDialogueDriver : MonoBehaviour
{
    [SerializeField]
    private ReciteDialogueRunner runner;

    private bool hasRelayKey;
    private ReciteSessionSnapshot savedSnapshot;

    private void Awake()
    {
        runner.Service.RegisterCondition("has_key", args => hasRelayKey);
    }

    private void Start()
    {
        StartDialogue();
    }

    public void StartDialogue()
    {
        runner.StartDialogue();
    }

    public void Choose(string choiceId)
    {
        runner.SelectChoice(choiceId);
    }

    public void CompleteBlockingEffect(string effectRequestId)
    {
        runner.AcknowledgeEffect(effectRequestId);
    }

    public void SaveSnapshot()
    {
        savedSnapshot = runner.Snapshot();
    }

    public void RestoreSnapshot()
    {
        if (savedSnapshot != null)
        {
            runner.Restore(savedSnapshot);
        }
    }

    public void OnReciteOutput(ReciteOutput output)
    {
        if (output is ReciteEffectOutput effectOutput)
        {
            Debug.Log(effectOutput.Effect.Mode + " effect: " + effectOutput.Effect.Function);
            if (effectOutput.Effect.Function == "grant_item")
            {
                hasRelayKey = true;
                runner.AcknowledgeEffect(effectOutput.Effect.Id);
            }
        }

        Debug.Log(output.Kind);
    }

    public void OnReciteError(ReciteAdapterException error)
    {
        Debug.LogError(error.Status + ": " + error.Message);
    }
}
