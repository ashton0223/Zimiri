extern crate image;
extern crate rand;

mod image_fun;

use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::time::Duration;
use std::collections::HashMap;
use std::fs;

use image::DynamicImage;
use image::ImageError;


use rand::{thread_rng, Rng};

use serenity::prelude::SerenityError;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::channel::Message;
use serenity::http::AttachmentType;
use serenity::framework::standard::{
    StandardFramework,
    CommandResult,
    macros::{
        command,
        group
    }
};

pub use image_fun::{rotate_image, overlay_bi_flag, vec_image};

#[derive(Debug)]
struct ZimiriError {
    description: String
}

impl ZimiriError {
    fn new(msg: &str) -> ZimiriError {
        ZimiriError{description: msg.to_string()}
    }
}

impl fmt::Display for ZimiriError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl Error for ZimiriError {
    fn description(&self) -> &str {
        &self.description
    }
}

impl From<SerenityError> for ZimiriError {
    fn from(err: SerenityError) -> Self {
        ZimiriError::new(&err.to_string())
    }
}

impl From<ImageError> for ZimiriError {
    fn from(err: ImageError) -> Self {
        ZimiriError::new(&err.to_string())
    }
}

#[group]
#[commands(ping, repeat, rotate, bi, invert, rps)]
struct General;

struct Handler;


#[async_trait]
impl EventHandler for Handler {
    /* Keep this as an example
    async fn message(&self, ctx: Context, msg: Message) {
        
        if msg.content == "!ping" {

            // sending message can fail
            if let Err(reason) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {:?}", reason)
            }
        }
    }*/
}


#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from a file
    let token = fs::read_to_string("TOKEN").expect("Couldn't read file");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn rps(ctx: &Context, msg: &Message) -> CommandResult {
    let mut game_message: String;
    let mut reaction_emoji = ' ';
    let bot_position: i32;
    // placeholder
    let mut first_bot_message = msg.clone();

    // HashMap to know who won in rock, paper, scissors
    let mut winner = HashMap::new();
    // Rock beats scissors, scissors beats paper, paper beats rock
    winner.insert(0, 2);
    winner.insert(2, 1);
    winner.insert(1, 0);
    // Rock, paper, and scissors emojis
    let emojis = vec!['🪨', '📄', '✂'];
    msg.channel_id.send_message(ctx, |m| {
        m.content("Rock, Paper, or Scissors?");
        m.reactions(emojis.clone().into_iter());

        m
    }).await?;

    // rnd can't exist across an await, so it must be done here
    // so that the compiler knows that the value is dropped.
    {
        let mut rng = thread_rng();
        bot_position = rng.gen_range(0..3);
    }

    if let Some(reaction) = &msg.author.await_reaction(&ctx).timeout(Duration::from_secs(10)).await {
        // Gets the char from the reaction.
        // Unwrapping is fine here because if there is a reaction
        // there must be an emoji.
        reaction_emoji = reaction.as_inner_ref().emoji.as_data().chars().next().unwrap();
        
        let position = char_in_vec(&emojis, reaction_emoji);

        // Position will never be -1, so it is returned if not
        // rock, paper, or scissors.
        if position != -1 {
            if bot_position == position {
                game_message = "Tie!".to_string();
            } else {
                let losing_position = winner[&position];
                if losing_position == bot_position {
                    game_message = "You won!".to_string();
                } else {
                    game_message = "You Lost!".to_string();
                }
            }
        } else {
            game_message = "Cheater!".to_string();
        }

        // Delete first bot message
        first_bot_message = reaction.as_inner_ref().message(ctx).await?;
    } else {
        game_message = "No pick in 10 seconds, quitting game.".to_string();
    }

    // Edit original message to show winner and 
    first_bot_message.delete_reactions(ctx).await?;

    let bot_message_text = emojis[bot_position as usize].to_string();
    game_message = format!("{} vs. {}\n{}", reaction_emoji, bot_message_text, game_message);

    first_bot_message.edit(ctx, |m| {
        m.content(game_message);

        m
    }).await?;

    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;
    Ok(())
}

#[command]
async fn repeat(ctx: &Context, msg: &Message) -> CommandResult {
    let fixed_msg = str::replace(
        msg.content.clone().as_str(),
        "!repeat", ""
    );
    msg.reply(ctx, fixed_msg).await?;
    Ok(())
}

#[command]
async fn rotate(ctx: &Context, msg: &Message) -> CommandResult {
    modify_single_image(ctx, msg, rotate_image).await?;
    Ok(())
}

#[command]
async fn bi(ctx: &Context, msg: &Message) -> CommandResult {
    modify_single_image(ctx, msg, overlay_bi_flag).await?;
    Ok(())
}

#[command]
async fn invert(ctx: &Context, msg: &Message) -> CommandResult {
    modify_single_image(ctx, msg, invert_image).await?;
    Ok(())
}

fn invert_image(img: DynamicImage) -> DynamicImage {
    let mut inverted = img.clone();
    inverted.invert();

    inverted
}

async fn modify_single_image(
    ctx: &Context, 
    msg: &Message, 
    operation: fn(img: DynamicImage) -> DynamicImage,
) -> Result<(), ZimiriError> {
    // Are there any attachments?
    if msg.attachments.len() == 0 {
        msg.reply(ctx, "No image(s) provided.").await?;
    } else {
        for attachment in &msg.attachments {
            // Are they images?
            if attachment.height.is_some() {
                start_typing(&ctx, &msg).await?;

                let image_file = attachment.download().await?;
                
                let image = image::load_from_memory(&image_file)?;
                let image = operation(image);

                send_image_message(ctx, &image, msg).await?;
            }
        }
    }
    Ok(())
}

fn process_vec(vec: &Vec<u8>) -> AttachmentType {
    let slice = vec.as_slice();
    let borrowed_slice = Cow::Borrowed(slice);
    
    AttachmentType::Bytes {
        data: borrowed_slice,
        filename: "output.png".to_string(),
    }
}

async fn send_image_message(ctx: &Context, img: &DynamicImage, msg: &Message) -> Result<(), ZimiriError> {
    let vec = match vec_image(img) {
        Ok(vec) => vec,
        Err(_e) => {        
            // Just returning the error breaks everything for some reason
            return Err(ZimiriError::new("Unable to rotate image"));
        },
    };
    let file = process_vec(&vec);
    msg.channel_id.send_message(ctx, |m| {
        m.content("");
        m.add_file(file);

        m
    }).await?;

    Ok(())
}

async fn start_typing(ctx: &Context, msg: &Message) -> Result<(), ZimiriError> {
    let id = msg.channel_id;
    id.broadcast_typing(ctx).await?;
    Ok(())
}

fn char_in_vec(vec: &Vec<char>, chr: char) -> i32 {
    for (i, &v_chr) in vec.into_iter().enumerate() {
        if chr == v_chr {
            return i as i32;
        }
    }

    -1
}