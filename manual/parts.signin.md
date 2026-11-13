# Sign-in parts

The parts of the Sign-in group, which stand on the sign-in views of the sign-in window: the fields, the sign-in buttons, keep me signed in, the language dropdown, the other way and the refusal.

## The sign-in window and its views

People who sign in on the desktop or the web see a window of the kind **Sign in** first (`<SignIn>`). It is built as every window is: window, content, views. Its content shows the sign-in views, one at a time; the first it names opens first. The window holds that content and decoration: a heading, a label, a picture, a line or a link. A sign-in view holds only the sign-in parts and that decoration, and each sign-in part stands on a sign-in view only. Switching between ways of signing in is switching views.

What the sign-in parts sign in with is the project's `signin` block, set under **Who is using it** in Project settings; see [Signing in](project.signin.md). Each part keeps only its place and look, and shows Clarity's own words, translated, unless its caption says others. Its properties say what it takes, chosen from the setup: the input of a field, the strategy of a button, the view an other way shows.

On a sign-in view the palette offers only the parts of this group, made from the setup, and the decoration: the language dropdown when the setup has **Language** ticked, a field per input of the strategies (**Name**, **Password**, **Recovery code**, **Token**), **Keep me signed in** when the setup remembers people only when asked, a button per strategy (**Sign in with a password**, **Use this computer's account**, **Sign in with GitHub**, ...), **Other way**, **Refusal**, and **Heading**, **Label**, **Picture**, **Separator** and **Link**.

**New window** offers the sign-in window and makes it from the setup: a view per strategy that takes typed inputs, with the language dropdown, the fields, keep me signed in, its button, the other ways and the refusal; a strategy that takes nothing typed, such as the computer's account, is a button on the first view; and an other way between each two views.

```xml
<!-- club/design/SignIn.cxaml -->
<SignIn class="club.SignIn" title="@signInTitle">
  <Heading name="signInHeading" at="1,1" span="12" caption="@clubTitle" />
  <Content name="ways" at="1,2" span="12" rows="10">
    <View name="PasswordView" />
    <View name="CodeView" />
  </Content>
</SignIn>
```

```xml
<!-- club/design/PasswordView.cxaml -->
<View class="club.PasswordView">
  <SigninField name="nameField" at="1,1" span="12" rows="2" input="name" />
  <SigninField name="passwordField" at="1,3" span="12" rows="2" input="password" />
  <SigninButton name="signInButton" at="1,5" span="12" strategy="password" />
  <SigninOther name="useCode" at="1,6" span="12" shows="CodeView" />
  <SigninRefusal name="refusal" at="1,7" span="12" />
</View>
```

```xml
<!-- club/design/CodeView.cxaml -->
<View class="club.CodeView">
  <SigninField name="codeName" at="1,1" span="12" rows="2" input="name" />
  <SigninField name="code" at="1,3" span="12" rows="2" input="code" />
  <SigninButton name="codeButton" at="1,5" span="12" strategy="recovery" />
  <SigninOther name="back" at="1,6" span="12" shows="PasswordView" />
</View>
```

A view has each part at most once: a field per input, a button per strategy. The checker refuses:

- a sign-in part anywhere but on a sign-in view ("belongs on a sign-in view, a view the sign-in window's content shows"), and on the sign-in window itself ("put the ... on a sign-in view");
- any other part on a sign-in view than the sign-in parts and a heading, label, picture, line or link;
- a part a view already has ("a sign-in view has one");
- a sign-in window without `signin` in the project file, a second sign-in window ("a project has one"), `allow` on it ("it shows before anyone signs in"), and a sign-in window together with a splash that signs in ("keep one").

The `signin` part on the app's splash, which draws the whole sign-in by itself, is in the Basic input group; see [Signing in](project.signin.md).

## The parts

### `signinField`

A label and a field for one input of a strategy; a password box for `password`. Its label is Clarity's `signinName`, `signinPassword`, `signinCode` or `signinToken` ("Name", "Password", "Recovery code", "Token").

```xml
<SigninField name="name" at="col,row" span="n" rows="2" input="input" caption="@key" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `input` | `name`, `password`, `code` or `token`, an input of the block's strategies | none: it is required | The input it takes (**Input**). |

It has no events.

```xml
<SigninField name="passwordField" at="1,3" span="12" rows="2" input="password" />
```

The checker refuses a field without `input` ("says which input it takes: input name, password or ..."), and an input none of the project's strategies take ("the project's strategies take name, password; not token").

### `signinButton`

Signs in with a strategy of the block, with what was typed in the sign-in fields of its own view; one that takes nothing typed (`account`, a provider) signs in at once. It says Clarity's `signinButton` ("Sign in"), `signinAccountButton` ("Use this computer's account"), or for a provider `signinGithub`, `signinMicrosoft` or `signinOpenid` ("Sign in with GitHub", "Sign in with Microsoft", "Sign in with your account").

```xml
<SigninButton name="name" at="col,row" span="n" strategy="strategy" caption="@key" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `strategy` | a strategy of the block: `password`, `recovery`, `token`, `account`, `github`, `microsoft`, `openid` | none: it is required | The strategy it signs in with (**Signs in with**). |

It has no events.

```xml
<SigninButton name="accountButton" at="1,9" span="12" strategy="account" />
```

The checker refuses a button without `strategy`, a strategy the project does not sign in with ("the project signs in with password, account; not token"), and a button whose view lacks a field for an input of its strategy ("signs in with password, which takes the password from a field on PasswordView (input password); it has none").

### `signinRemember`

"Keep me signed in" (Clarity's `signinRemember`): a box the person ticks to have the program keep the sign-in, for a block with `remember 30 days when asked`.

```xml
<SigninRemember name="remember" at="col,row" span="n" />
```

It has no properties of its own and no events. The checker refuses it when the block does not remember only when asked ("needs remember 30 days when asked in the signin block").

### `signinLanguage`

A dropdown of the program's languages, each by its own name, headed by Clarity's `languagePicker`; picking one shows every text in it. It needs `language picker` in the block.

```xml
<SigninLanguage name="language" at="col,row" span="n" rows="2" />
```

It has no properties of its own and no events. The checker refuses it when the block has no `language picker` ("the language dropdown needs language picker in the signin block").

### `signinOther`

A link that shows another sign-in view: another way to sign in. It says Clarity's `signinUseCode` ("Use a recovery code") when the view it shows signs in with a recovery code, `signinUseToken` ("Use a token") with a token, and `signinUsePassword` ("Sign in with a password") otherwise.

```xml
<SigninOther name="name" at="col,row" span="n" shows="View" caption="@key" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `shows` | a view the sign-in window's content shows | none: it is required | The view it shows (**Shows**). |

It has no events.

```xml
<SigninOther name="useCode" at="1,6" span="12" shows="CodeView" />
```

The checker refuses an other way without `shows`, one that shows the view it stands on, and a view the content of the sign-in window does not show ("shows a sign-in view, one the content of SignIn shows: PasswordView, CodeView; not ...").

### `signinRefusal`

Where a refused sign-in says why. Without one on the view, a refusal shows as a message.

```xml
<SigninRefusal name="refusal" at="col,row" span="n" />
```

It has no properties of its own and no events.

## See also

- [Signing in](project.signin.md): the `signin` block, the strategies, the splash's sign-in part and who signed in, in code.
- [Windows](design.windows.md): windows, content and views.
- [Parts](design.parts.md): the Parts palette and the properties panel.
- [Languages and formats](languages.md): the translations of Clarity's own words.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Field | `signinField` | nothing | 6 x 2 | no | `input` | none |
| Sign-in button | `signinButton` | nothing | 6 x 1 | no | `strategy` | none |
| Keep me signed in | `signinRemember` | nothing | 6 x 1 | no | none | none |
| Language | `signinLanguage` | nothing | 6 x 2 | no | none | none |
| Other way | `signinOther` | nothing | 6 x 1 | no | `shows` | none |
| Refusal | `signinRefusal` | nothing | 6 x 1 | no | none | none |

<!-- /catalog -->
