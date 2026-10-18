// A record of an entity, as generated code describes it.

namespace Clarity.Runtime;

public interface IRecord<TSelf> where TSelf : IRecord<TSelf>
{
    /// <summary>The data source the records live in; empty for the default one.</summary>
    static virtual string SourceName => "";

    /// <summary>The table the records are stored in.</summary>
    static abstract string TableName { get; }

    /// <summary>The integer column that identifies a record.</summary>
    static virtual string KeyColumn => "id";

    /// <summary>The stored columns, without the key, in the order of <see cref="ToValues"/>.</summary>
    static abstract IReadOnlyList<string> ColumnNames { get; }

    /// <summary>
    /// The columns of the image fields, which reading records gives only
    /// whether there is a picture, and saving leaves alone when the picture is
    /// the one stored.
    /// </summary>
    static virtual IReadOnlyList<PictureColumn> Pictures => [];

    /// <summary>
    /// Whether the entity has data events. A save or delete then runs with its
    /// events in one transaction: when an event fails, nothing of it is kept.
    /// </summary>
    static virtual bool HasEvents => false;

    /// <summary>The key of a stored record; 0 until it is saved for the first time.</summary>
    long Id { get; set; }

    object?[] ToValues();

    /// <summary>Builds a record from values in <see cref="ColumnNames"/> order.</summary>
    static abstract TSelf FromValues(long id, object?[] values);

    void BeforeSave(Db db) { }

    void AfterSave(Db db) { }

    void BeforeDelete(Db db) { }

    void AfterDelete(Db db) { }
}
